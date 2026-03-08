use async_graphql::{Context, EmptySubscription, Object, Schema};
use send_wrapper::SendWrapper;

use crate::http_client::FlightApi;
use crate::models::{CreateFlightInput, Flight};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn flight(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<Flight>> {
        let client = ctx.data::<Box<dyn FlightApi>>()?;
        match SendWrapper::new(client.get_flight(id)).await {
            Ok(flight) => Ok(Some(flight)),
            Err(e) if e.is_not_found() => Ok(None),
            Err(e) => Err(e.to_string().into()),
        }
    }

    async fn flights(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<Vec<Flight>> {
        let client = ctx.data::<Box<dyn FlightApi>>()?;
        let limit = limit.unwrap_or(20).clamp(0, 100);
        let offset = offset.unwrap_or(0).max(0);
        SendWrapper::new(client.get_flights(limit, offset))
            .await
            .map_err(|e| e.to_string().into())
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_flight(
        &self,
        ctx: &Context<'_>,
        input: CreateFlightInput,
    ) -> async_graphql::Result<Flight> {
        let client = ctx.data::<Box<dyn FlightApi>>()?;
        SendWrapper::new(client.create_flight(input))
            .await
            .map_err(|e| e.to_string().into())
    }
}

pub type FlightSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(client: Box<dyn FlightApi>) -> FlightSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(client)
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::OriginError;
    use async_graphql::Request;
    use std::future::Future;
    use std::pin::Pin;

    struct MockFlightApi {
        flights: Vec<Flight>,
    }

    impl MockFlightApi {
        fn new(flights: Vec<Flight>) -> Self {
            Self { flights }
        }
    }

    impl FlightApi for MockFlightApi {
        fn get_flight(
            &self,
            id: String,
        ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>> {
            let result = self
                .flights
                .iter()
                .find(|f| f.id == id)
                .cloned()
                .ok_or(OriginError::Status(404));
            Box::pin(async move { result })
        }

        fn get_flights(
            &self,
            limit: i32,
            offset: i32,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<Flight>, OriginError>> + '_>> {
            let result: Vec<Flight> = self
                .flights
                .iter()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect();
            Box::pin(async move { Ok(result) })
        }

        fn create_flight(
            &self,
            input: CreateFlightInput,
        ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>> {
            let flight = Flight {
                id: "new-1".to_string(),
                date: input.date,
                aircraft_title: input.aircraft_title,
                aircraft_registration: input.aircraft_registration,
                departure_icao: input.departure_icao,
                departure_name: input.departure_name,
                departure_lat: input.departure_lat,
                departure_lon: input.departure_lon,
                arrival_icao: input.arrival_icao,
                arrival_name: input.arrival_name,
                arrival_lat: input.arrival_lat,
                arrival_lon: input.arrival_lon,
                distance_nm: input.distance_nm,
                elapsed_seconds: input.elapsed_seconds,
                max_altitude_ft: input.max_altitude_ft,
                landing_vs_fpm: input.landing_vs_fpm,
                landing_g_force: input.landing_g_force,
                notes: input.notes,
            };
            Box::pin(async move { Ok(flight) })
        }
    }

    /// Always returns Err for any call — simulates origin failures.
    struct FailingFlightApi {
        status: u16,
    }

    impl FlightApi for FailingFlightApi {
        fn get_flight(
            &self,
            _id: String,
        ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>> {
            Box::pin(async { Err(OriginError::Status(self.status)) })
        }

        fn get_flights(
            &self,
            _limit: i32,
            _offset: i32,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<Flight>, OriginError>> + '_>> {
            Box::pin(async { Err(OriginError::Status(self.status)) })
        }

        fn create_flight(
            &self,
            _input: CreateFlightInput,
        ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>> {
            Box::pin(async { Err(OriginError::Status(self.status)) })
        }
    }

    fn make_flight(id: &str, date: &str) -> Flight {
        Flight {
            id: id.to_string(),
            date: date.to_string(),
            aircraft_title: None,
            aircraft_registration: None,
            departure_icao: None,
            departure_name: None,
            departure_lat: None,
            departure_lon: None,
            arrival_icao: None,
            arrival_name: None,
            arrival_lat: None,
            arrival_lon: None,
            distance_nm: None,
            elapsed_seconds: None,
            max_altitude_ft: None,
            landing_vs_fpm: None,
            landing_g_force: None,
            notes: None,
        }
    }

    fn test_schema(mock: MockFlightApi) -> FlightSchema {
        build_schema(Box::new(mock))
    }

    fn failing_schema(status: u16) -> FlightSchema {
        build_schema(Box::new(FailingFlightApi { status }))
    }

    // --- Schema introspection tests ---

    #[tokio::test]
    async fn schema_has_flight_query() {
        let schema = test_schema(MockFlightApi::new(vec![]));
        let resp = schema
            .execute(Request::new(
                r#"{ __type(name: "QueryRoot") { fields { name } } }"#,
            ))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        let fields: Vec<&str> = json["__type"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert!(fields.contains(&"flight"));
        assert!(fields.contains(&"flights"));
    }

    #[tokio::test]
    async fn schema_has_create_flight_mutation() {
        let schema = test_schema(MockFlightApi::new(vec![]));
        let resp = schema
            .execute(Request::new(
                r#"{ __type(name: "MutationRoot") { fields { name } } }"#,
            ))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        let fields: Vec<&str> = json["__type"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert!(fields.contains(&"createFlight"));
    }

    #[tokio::test]
    async fn flight_type_has_expected_fields() {
        let schema = test_schema(MockFlightApi::new(vec![]));
        let resp = schema
            .execute(Request::new(
                r#"{ __type(name: "Flight") { fields { name } } }"#,
            ))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        let fields: Vec<&str> = json["__type"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        for expected in [
            "id",
            "date",
            "aircraftTitle",
            "departureIcao",
            "arrivalIcao",
            "distanceNm",
            "elapsedSeconds",
            "notes",
        ] {
            assert!(fields.contains(&expected), "missing field: {expected}");
        }
    }

    // --- Resolver tests ---

    #[tokio::test]
    async fn flight_returns_some_when_found() {
        let schema = test_schema(MockFlightApi::new(vec![make_flight("f1", "2026-01-01")]));
        let resp = schema
            .execute(Request::new(r#"{ flight(id: "f1") { id date } }"#))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        assert_eq!(json["flight"]["id"], "f1");
        assert_eq!(json["flight"]["date"], "2026-01-01");
    }

    #[tokio::test]
    async fn flight_returns_null_for_404() {
        let schema = test_schema(MockFlightApi::new(vec![]));
        let resp = schema
            .execute(Request::new(r#"{ flight(id: "missing") { id } }"#))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        assert!(json["flight"].is_null());
    }

    #[tokio::test]
    async fn flight_returns_error_for_non_404() {
        let schema = failing_schema(500);
        let resp = schema
            .execute(Request::new(r#"{ flight(id: "f1") { id } }"#))
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("500"));
    }

    #[tokio::test]
    async fn flights_returns_list() {
        let schema = test_schema(MockFlightApi::new(vec![
            make_flight("f1", "2026-01-01"),
            make_flight("f2", "2026-01-02"),
        ]));
        let resp = schema
            .execute(Request::new("{ flights { id } }"))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        let ids: Vec<&str> = json["flights"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, vec!["f1", "f2"]);
    }

    #[tokio::test]
    async fn flights_respects_limit_and_offset() {
        let flights: Vec<Flight> = (0..5)
            .map(|i| make_flight(&format!("f{i}"), "2026-01-01"))
            .collect();
        let schema = test_schema(MockFlightApi::new(flights));
        let resp = schema
            .execute(Request::new(
                "{ flights(limit: 2, offset: 1) { id } }",
            ))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        let ids: Vec<&str> = json["flights"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, vec!["f1", "f2"]);
    }

    #[tokio::test]
    async fn create_flight_returns_created_flight() {
        let schema = test_schema(MockFlightApi::new(vec![]));
        let resp = schema
            .execute(Request::new(
                r#"mutation { createFlight(input: { date: "2026-03-08" }) { id date } }"#,
            ))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        assert_eq!(json["createFlight"]["id"], "new-1");
        assert_eq!(json["createFlight"]["date"], "2026-03-08");
    }
}
