use async_graphql::{InputObject, SimpleObject};
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
