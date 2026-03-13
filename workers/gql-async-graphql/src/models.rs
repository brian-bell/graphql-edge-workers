use async_graphql::{InputObject, SimpleObject, ID};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct FlightRow {
    pub id: String,
    #[allow(dead_code)]
    pub user_id: String,
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

impl From<FlightRow> for Flight {
    fn from(row: FlightRow) -> Self {
        Self {
            id: row.id.into(),
            date: row.date,
            aircraft_title: row.aircraft_title,
            aircraft_registration: row.aircraft_registration,
            departure_icao: row.departure_icao,
            departure_name: row.departure_name,
            departure_lat: row.departure_lat,
            departure_lon: row.departure_lon,
            arrival_icao: row.arrival_icao,
            arrival_name: row.arrival_name,
            arrival_lat: row.arrival_lat,
            arrival_lon: row.arrival_lon,
            distance_nm: row.distance_nm,
            elapsed_seconds: row.elapsed_seconds,
            max_altitude_ft: row.max_altitude_ft,
            landing_vs_fpm: row.landing_vs_fpm,
            landing_g_force: row.landing_g_force,
            notes: row.notes,
        }
    }
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

#[derive(Debug, Serialize)]
pub struct CreateFlightPayload {
    pub user_id: String,
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

impl CreateFlightPayload {
    pub fn new(input: CreateFlightInput, user_id: String) -> Self {
        Self {
            user_id,
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
        }
    }
}

#[derive(Debug, Serialize, InputObject)]
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

#[derive(Debug, Serialize)]
pub struct UpdateFlightPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aircraft_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aircraft_registration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub departure_icao: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub departure_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub departure_lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub departure_lon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrival_icao: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrival_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrival_lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrival_lon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_nm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_seconds: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_altitude_ft: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landing_vs_fpm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landing_g_force: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl From<UpdateFlightInput> for UpdateFlightPatch {
    fn from(input: UpdateFlightInput) -> Self {
        Self {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateFlightInput, CreateFlightPayload, Flight, FlightRow, UpdateFlightPatch};

    #[test]
    fn patch_serialization_omits_absent_fields() {
        let patch = UpdateFlightPatch {
            date: Some("2026-03-12".to_string()),
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
            notes: Some("Updated".to_string()),
        };

        let value = serde_json::to_value(&patch).unwrap();
        let object = value.as_object().unwrap();

        assert_eq!(object.len(), 2);
        assert_eq!(object.get("date").unwrap(), "2026-03-12");
        assert_eq!(object.get("notes").unwrap(), "Updated");
        assert!(!object.contains_key("aircraft_title"));
    }

    #[test]
    fn create_payload_includes_user_id() {
        let payload = CreateFlightPayload::new(
            CreateFlightInput {
                date: "2026-03-12".to_string(),
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
                notes: Some("First flight".to_string()),
            },
            "user-123".to_string(),
        );

        let value = serde_json::to_value(&payload).unwrap();
        assert_eq!(value["user_id"], "user-123");
        assert_eq!(value["notes"], "First flight");
    }

    #[test]
    fn deserializes_supabase_row_with_snake_case_fields() {
        let row: FlightRow = serde_json::from_value(serde_json::json!({
            "id": "flight-1",
            "user_id": "user-123",
            "date": "2026-03-12",
            "aircraft_title": "C172",
            "notes": "Smooth landing"
        }))
        .unwrap();

        assert_eq!(row.user_id, "user-123");

        let flight: Flight = row.into();
        assert_eq!(flight.id.as_str(), "flight-1");
        assert_eq!(flight.aircraft_title.as_deref(), Some("C172"));
        assert_eq!(flight.notes.as_deref(), Some("Smooth landing"));
    }
}
