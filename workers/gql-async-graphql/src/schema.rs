use async_graphql::{Context, EmptySubscription, InputObject, Object, Schema, SimpleObject};
use send_wrapper::SendWrapper;
use serde::{Deserialize, Serialize};

use crate::http_client::OriginClient;

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

#[derive(Debug, Serialize, InputObject)]
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
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<Flight>> {
        let client = ctx.data::<OriginClient>()?;
        let path = format!("/flights/{id}");
        match SendWrapper::new(client.get::<Flight>(&path)).await {
            Ok(flight) => Ok(Some(flight)),
            Err(e) if e.contains("404") => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn flights(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<Vec<Flight>> {
        let client = ctx.data::<OriginClient>()?;
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);
        let path = format!("/flights?limit={limit}&offset={offset}");
        SendWrapper::new(client.get::<Vec<Flight>>(&path))
            .await
            .map_err(|e| e.into())
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
        let client = ctx.data::<OriginClient>()?;
        SendWrapper::new(client.post::<Flight, _>("/flights", &input))
            .await
            .map_err(|e| e.into())
    }
}

pub type FlightSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(origin_client: OriginClient) -> FlightSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(origin_client)
        .finish()
}
