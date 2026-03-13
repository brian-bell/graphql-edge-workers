use async_graphql::{Context, EmptySubscription, Object, Schema, ID};
use send_wrapper::SendWrapper;

use crate::http_client::FlightApi;
use crate::models::{CreateFlightInput, Flight, UpdateFlightInput};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn flight(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> async_graphql::Result<Option<Flight>> {
        let client = ctx.data::<Box<dyn FlightApi>>()?;
        match SendWrapper::new(client.get_flight(id.to_string())).await {
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

    async fn update_flight(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateFlightInput,
    ) -> async_graphql::Result<Flight> {
        let client = ctx.data::<Box<dyn FlightApi>>()?;
        SendWrapper::new(client.update_flight(id.to_string(), input))
            .await
            .map_err(|e| e.to_string().into())
    }

    async fn delete_flight(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> async_graphql::Result<bool> {
        let client = ctx.data::<Box<dyn FlightApi>>()?;
        SendWrapper::new(client.delete_flight(id.to_string()))
            .await
            .map(|()| true)
            .map_err(|e| e.to_string().into())
    }
}

pub type FlightSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_base_schema() -> FlightSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish()
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn build_schema(client: Box<dyn FlightApi>) -> FlightSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(client)
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::OriginError;
    use crate::models::UpdateFlightInput;
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
                .find(|f| f.id.as_str() == id)
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
                id: "new-1".into(),
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

        fn update_flight(
            &self,
            id: String,
            input: UpdateFlightInput,
        ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>> {
            let result = self
                .flights
                .iter()
                .find(|f| f.id.as_str() == id)
                .cloned()
                .map(|mut f| {
                    if let Some(date) = input.date {
                        f.date = date;
                    }
                    if input.notes.is_some() {
                        f.notes = input.notes;
                    }
                    f
                })
                .ok_or(OriginError::Status(404));
            Box::pin(async move { result })
        }

        fn delete_flight(
            &self,
            id: String,
        ) -> Pin<Box<dyn Future<Output = Result<(), OriginError>> + '_>> {
            let exists = self.flights.iter().any(|f| f.id.as_str() == id);
            Box::pin(async move {
                if exists {
                    Ok(())
                } else {
                    Err(OriginError::Status(404))
                }
            })
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

        fn update_flight(
            &self,
            _id: String,
            _input: UpdateFlightInput,
        ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>> {
            Box::pin(async { Err(OriginError::Status(self.status)) })
        }

        fn delete_flight(
            &self,
            _id: String,
        ) -> Pin<Box<dyn Future<Output = Result<(), OriginError>> + '_>> {
            Box::pin(async { Err(OriginError::Status(self.status)) })
        }
    }

    fn make_flight(id: &str, date: &str) -> Flight {
        Flight {
            id: id.into(),
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
    async fn schema_has_all_mutations() {
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
        assert!(fields.contains(&"updateFlight"));
        assert!(fields.contains(&"deleteFlight"));
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

    #[tokio::test]
    async fn flight_id_is_graphql_id_scalar() {
        let schema = test_schema(MockFlightApi::new(vec![]));
        let resp = schema
            .execute(Request::new(
                r#"{ __type(name: "Flight") { fields { name type { kind ofType { name } } } } }"#,
            ))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        let id_field = json["__type"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == "id")
            .expect("id field not found");
        // id is ID! (NON_NULL wrapping the ID scalar)
        assert_eq!(id_field["type"]["kind"], "NON_NULL");
        assert_eq!(id_field["type"]["ofType"]["name"], "ID");
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
    async fn flights_clamps_negative_limit_to_zero() {
        let flights: Vec<Flight> = (0..3)
            .map(|i| make_flight(&format!("f{i}"), "2026-01-01"))
            .collect();
        let schema = test_schema(MockFlightApi::new(flights));
        let resp = schema
            .execute(Request::new("{ flights(limit: -5) { id } }"))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        let arr = json["flights"].as_array().unwrap();
        assert_eq!(arr.len(), 0);
    }

    #[tokio::test]
    async fn flights_clamps_limit_to_max_100() {
        let flights: Vec<Flight> = (0..3)
            .map(|i| make_flight(&format!("f{i}"), "2026-01-01"))
            .collect();
        let schema = test_schema(MockFlightApi::new(flights));
        // Request limit=999 but mock only has 3 items — the point is
        // that the value forwarded to the API is clamped to 100, not 999.
        let resp = schema
            .execute(Request::new("{ flights(limit: 999) { id } }"))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        let arr = json["flights"].as_array().unwrap();
        // Mock has only 3, so we get 3 (limit=100 > 3)
        assert_eq!(arr.len(), 3);
    }

    #[tokio::test]
    async fn flights_clamps_negative_offset_to_zero() {
        let flights: Vec<Flight> = (0..3)
            .map(|i| make_flight(&format!("f{i}"), "2026-01-01"))
            .collect();
        let schema = test_schema(MockFlightApi::new(flights));
        let resp = schema
            .execute(Request::new("{ flights(offset: -10) { id } }"))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        let arr = json["flights"].as_array().unwrap();
        // offset clamped to 0, default limit=20, so all 3 returned
        assert_eq!(arr.len(), 3);
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

    #[tokio::test]
    async fn update_flight_returns_updated_flight() {
        let schema = test_schema(MockFlightApi::new(vec![make_flight("f1", "2026-01-01")]));
        let resp = schema
            .execute(Request::new(
                r#"mutation { updateFlight(id: "f1", input: { date: "2026-06-15", notes: "updated" }) { id date notes } }"#,
            ))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        assert_eq!(json["updateFlight"]["id"], "f1");
        assert_eq!(json["updateFlight"]["date"], "2026-06-15");
        assert_eq!(json["updateFlight"]["notes"], "updated");
    }

    #[tokio::test]
    async fn update_flight_returns_error_for_missing() {
        let schema = test_schema(MockFlightApi::new(vec![]));
        let resp = schema
            .execute(Request::new(
                r#"mutation { updateFlight(id: "missing", input: { date: "2026-06-15" }) { id } }"#,
            ))
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("404"));
    }

    #[tokio::test]
    async fn delete_flight_returns_true() {
        let schema = test_schema(MockFlightApi::new(vec![make_flight("f1", "2026-01-01")]));
        let resp = schema
            .execute(Request::new(
                r#"mutation { deleteFlight(id: "f1") }"#,
            ))
            .await;
        assert!(resp.errors.is_empty());
        let json = resp.data.into_json().unwrap();
        assert_eq!(json["deleteFlight"], true);
    }

    #[tokio::test]
    async fn delete_flight_returns_error_for_missing() {
        let schema = test_schema(MockFlightApi::new(vec![]));
        let resp = schema
            .execute(Request::new(
                r#"mutation { deleteFlight(id: "missing") }"#,
            ))
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("404"));
    }
}
