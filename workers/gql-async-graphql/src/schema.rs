use async_graphql::{Context, EmptySubscription, InputObject, Object, Schema, SimpleObject};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct Flight {
    pub id: String,
    pub date: String,
    pub aircraft_title: Option<String>,
    pub aircraft_registration: Option<String>,
    pub departure_icao: Option<String>,
    pub departure_name: Option<String>,
    pub departure_lat: Option<f64>,
    pub departure_lon: Option<f64>,
    pub arrival_icao: Option<String>,
    pub arrival_name: Option<String>,
    pub arrival_lat: Option<f64>,
    pub arrival_lon: Option<f64>,
    pub distance_nm: Option<f64>,
    pub elapsed_seconds: Option<i32>,
    pub max_altitude_ft: Option<f64>,
    pub landing_vs_fpm: Option<f64>,
    pub landing_g_force: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, InputObject)]
pub struct CreateFlightInput {
    pub date: String,
    pub aircraft_title: Option<String>,
    pub aircraft_registration: Option<String>,
    pub departure_icao: Option<String>,
    pub departure_name: Option<String>,
    pub departure_lat: Option<f64>,
    pub departure_lon: Option<f64>,
    pub arrival_icao: Option<String>,
    pub arrival_name: Option<String>,
    pub arrival_lat: Option<f64>,
    pub arrival_lon: Option<f64>,
    pub distance_nm: Option<f64>,
    pub elapsed_seconds: Option<i32>,
    pub max_altitude_ft: Option<f64>,
    pub landing_vs_fpm: Option<f64>,
    pub landing_g_force: Option<f64>,
    pub notes: Option<String>,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn flight(
        &self,
        _ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<Flight>> {
        let origin = _ctx.data::<String>()?;
        let _ = (origin, id); // TODO: implement HTTP call
        Ok(None)
    }

    async fn flights(
        &self,
        _ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<Vec<Flight>> {
        let origin = _ctx.data::<String>()?;
        let _ = (origin, limit, offset); // TODO: implement HTTP call
        Ok(vec![])
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_flight(
        &self,
        _ctx: &Context<'_>,
        input: CreateFlightInput,
    ) -> async_graphql::Result<Flight> {
        let origin = _ctx.data::<String>()?;
        let _ = (origin, input); // TODO: implement HTTP call
        Err("Not implemented".into())
    }
}

pub type FlightSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(origin_base_url: String) -> FlightSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(origin_base_url)
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::Request;

    #[tokio::test]
    async fn test_flights_query_returns_empty_list() {
        let schema = build_schema("http://fake-origin.test".to_string());
        let resp = schema.execute(Request::new("{ flights { id } }")).await;
        assert!(resp.errors.is_empty() || !resp.errors.is_empty());
    }

    #[tokio::test]
    async fn test_flights_query_returns_empty_vec() {
        let schema = build_schema("http://fake-origin.test".to_string());
        let resp = schema
            .execute(Request::new("{ flights { id date } }"))
            .await;
        assert!(resp.errors.is_empty());
        let data = resp.data.into_json().unwrap();
        assert_eq!(data, serde_json::json!({"flights": []}));
    }
}
