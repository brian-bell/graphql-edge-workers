use async_graphql::{InputObject, SimpleObject, ID};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct Flight {
    pub id: ID,
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
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Serialize, InputObject)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFlightInput {
    pub date: Option<String>,
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
