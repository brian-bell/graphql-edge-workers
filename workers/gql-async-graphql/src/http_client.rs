use std::fmt;
use std::future::Future;
use std::pin::Pin;

use serde::de::DeserializeOwned;
use worker::{Fetch, Headers, Method, Request, RequestInit, Url};

use crate::auth::AuthContext;
use crate::config::RuntimeConfig;
use crate::models::{
    CreateFlightInput, CreateFlightPayload, Flight, FlightRow, UpdateFlightInput,
    UpdateFlightPatch,
};

#[derive(Debug, PartialEq, Eq)]
pub enum OriginError {
    Status(u16),
    Other(String),
}

impl OriginError {
    pub fn is_not_found(&self) -> bool {
        matches!(self, OriginError::Status(404))
    }
}

impl fmt::Display for OriginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OriginError::Status(code) => write!(f, "Origin returned status {code}"),
            OriginError::Other(message) => f.write_str(message),
        }
    }
}

pub trait FlightApi: Send + Sync {
    fn get_flight(
        &self,
        id: String,
    ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>>;
    fn get_flights(
        &self,
        limit: i32,
        offset: i32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Flight>, OriginError>> + '_>>;
    fn create_flight(
        &self,
        input: CreateFlightInput,
    ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>>;
    fn update_flight(
        &self,
        id: String,
        input: UpdateFlightInput,
    ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>>;
    fn delete_flight(
        &self,
        id: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), OriginError>> + '_>>;
}

pub struct SupabaseClient {
    config: RuntimeConfig,
    auth: AuthContext,
}

impl SupabaseClient {
    pub fn new(config: RuntimeConfig, auth: AuthContext) -> Self {
        Self { config, auth }
    }

    async fn get_rows(&self, query: FlightsQuery) -> Result<Vec<FlightRow>, OriginError> {
        self.execute_json::<Vec<FlightRow>>(self.prepare_get(query)?).await
    }

    async fn create_row(
        &self,
        input: CreateFlightInput,
    ) -> Result<Flight, OriginError> {
        let payload = CreateFlightPayload::new(input, self.auth.user_id.clone());
        let rows = self
            .execute_json::<Vec<FlightRow>>(self.prepare_write(Method::Post, FlightsQuery::select_all(), Some(&payload), true)?)
            .await?;

        rows.into_iter()
            .next()
            .map(Into::into)
            .ok_or(OriginError::Other("Supabase create returned no rows".to_string()))
    }

    async fn update_row(
        &self,
        id: String,
        input: UpdateFlightInput,
    ) -> Result<Flight, OriginError> {
        let patch = UpdateFlightPatch::from(input);
        let rows = self
            .execute_json::<Vec<FlightRow>>(self.prepare_write(
                Method::Patch,
                FlightsQuery::single(id),
                Some(&patch),
                true,
            )?)
            .await?;

        rows.into_iter()
            .next()
            .map(Into::into)
            .ok_or(OriginError::Status(404))
    }

    async fn delete_row(&self, id: String) -> Result<(), OriginError> {
        let rows = self
            .execute_json::<Vec<FlightRow>>(self.prepare_write::<serde_json::Value>(
                Method::Delete,
                FlightsQuery::single(id),
                None,
                true,
            )?)
            .await?;

        if rows.is_empty() {
            Err(OriginError::Status(404))
        } else {
            Ok(())
        }
    }

    fn prepare_get(&self, query: FlightsQuery) -> Result<PreparedRequest, OriginError> {
        let url = build_flights_url(&self.config, &query)?;
        Ok(PreparedRequest {
            method: Method::Get,
            url,
            headers: common_headers(&self.config, &self.auth, false, None),
            body: None,
        })
    }

    fn prepare_write<B: serde::Serialize>(
        &self,
        method: Method,
        query: FlightsQuery,
        body: Option<&B>,
        return_representation: bool,
    ) -> Result<PreparedRequest, OriginError> {
        let url = build_flights_url(&self.config, &query)?;
        let prefer = return_representation.then_some("return=representation");
        let body = body
            .map(|payload| {
                serde_json::to_string(payload).map_err(|e| {
                    OriginError::Other(format!("Failed to serialize request body: {e}"))
                })
            })
            .transpose()?;

        Ok(PreparedRequest {
            method,
            url,
            headers: common_headers(&self.config, &self.auth, body.is_some(), prefer),
            body,
        })
    }

    async fn execute_json<T: DeserializeOwned>(
        &self,
        prepared: PreparedRequest,
    ) -> Result<T, OriginError> {
        let mut response = self.send(prepared).await?;

        if response.status_code() >= 400 {
            return Err(OriginError::Status(response.status_code()));
        }

        let body = response
            .text()
            .await
            .map_err(|e| OriginError::Other(format!("Failed to read response body: {e}")))?;

        serde_json::from_str(&body)
            .map_err(|e| OriginError::Other(format!("Failed to parse response JSON: {e}")))
    }

    async fn send(&self, prepared: PreparedRequest) -> Result<worker::Response, OriginError> {
        let mut init = RequestInit::new();
        init.with_method(prepared.method);
        let headers = Headers::new();
        for (name, value) in &prepared.headers {
            headers
                .set(name, value)
                .map_err(|e| OriginError::Other(format!("Failed to set header {name}: {e}")))?;
        }
        init.with_headers(headers);

        if let Some(body) = prepared.body {
            init.with_body(Some(worker::wasm_bindgen::JsValue::from_str(&body)));
        }

        let request = Request::new_with_init(&prepared.url, &init)
            .map_err(|e| OriginError::Other(format!("Failed to create request: {e}")))?;

        Fetch::Request(request)
            .send()
            .await
            .map_err(|e| OriginError::Other(format!("Fetch failed: {e}")))
    }
}

impl FlightApi for SupabaseClient {
    fn get_flight(
        &self,
        id: String,
    ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>> {
        Box::pin(async move {
            let mut rows = self.get_rows(FlightsQuery::single(id)).await?;
            rows.pop().map(Into::into).ok_or(OriginError::Status(404))
        })
    }

    fn get_flights(
        &self,
        limit: i32,
        offset: i32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Flight>, OriginError>> + '_>> {
        Box::pin(async move {
            let rows = self
                .get_rows(FlightsQuery::list(limit, offset))
                .await?;
            Ok(rows.into_iter().map(Into::into).collect())
        })
    }

    fn create_flight(
        &self,
        input: CreateFlightInput,
    ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>> {
        Box::pin(async move { self.create_row(input).await })
    }

    fn update_flight(
        &self,
        id: String,
        input: UpdateFlightInput,
    ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>> {
        Box::pin(async move { self.update_row(id, input).await })
    }

    fn delete_flight(
        &self,
        id: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), OriginError>> + '_>> {
        Box::pin(async move { self.delete_row(id).await })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedRequest {
    method: Method,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

#[derive(Debug, Clone)]
struct FlightsQuery {
    id: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
}

impl FlightsQuery {
    fn select_all() -> Self {
        Self {
            id: None,
            limit: None,
            offset: None,
        }
    }

    fn single(id: String) -> Self {
        Self {
            id: Some(id),
            limit: Some(1),
            offset: Some(0),
        }
    }

    fn list(limit: i32, offset: i32) -> Self {
        Self {
            id: None,
            limit: Some(limit),
            offset: Some(offset),
        }
    }
}

fn common_headers(
    config: &RuntimeConfig,
    auth: &AuthContext,
    has_body: bool,
    prefer: Option<&str>,
) -> Vec<(String, String)> {
    let mut headers = vec![
        ("apikey".to_string(), config.supabase_publishable_key().to_string()),
        (
            "authorization".to_string(),
            format!("Bearer {}", auth.access_token),
        ),
    ];

    if has_body {
        headers.push(("content-type".to_string(), "application/json".to_string()));
    }

    if let Some(prefer) = prefer {
        headers.push(("prefer".to_string(), prefer.to_string()));
    }

    headers
}

fn build_flights_url(
    config: &RuntimeConfig,
    query: &FlightsQuery,
) -> Result<String, OriginError> {
    let mut url = Url::parse(&config.flights_rest_url())
        .map_err(|e| OriginError::Other(format!("Invalid Supabase URL: {e}")))?;

    {
        let mut params = url.query_pairs_mut();
        params.append_pair("select", "*");
        if let Some(id) = &query.id {
            params.append_pair("id", &format!("eq.{id}"));
        }
        if let Some(limit) = query.limit {
            params.append_pair("limit", &limit.to_string());
        }
        if let Some(offset) = query.offset {
            params.append_pair("offset", &offset.to_string());
        }
    }

    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RuntimeConfig {
        RuntimeConfig::new("https://example.supabase.co", "publishable-key")
    }

    fn test_auth() -> AuthContext {
        AuthContext {
            access_token: "token-123".to_string(),
            user_id: "user-123".to_string(),
        }
    }

    fn test_client() -> SupabaseClient {
        SupabaseClient::new(test_config(), test_auth())
    }

    #[test]
    fn create_request_includes_auth_headers_and_user_id() {
        let client = test_client();
        let prepared = client
            .prepare_write(
                Method::Post,
                FlightsQuery::select_all(),
                Some(&CreateFlightPayload::new(
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
                        notes: Some("hello".to_string()),
                    },
                    client.auth.user_id.clone(),
                )),
                true,
            )
            .unwrap();

        assert_eq!(prepared.method, Method::Post);
        assert!(prepared
            .headers
            .contains(&("apikey".to_string(), "publishable-key".to_string())));
        assert!(prepared.headers.contains(&(
            "authorization".to_string(),
            "Bearer token-123".to_string()
        )));
        assert!(prepared
            .body
            .as_deref()
            .unwrap()
            .contains("\"user_id\":\"user-123\""));
    }

    #[test]
    fn list_request_encodes_limit_and_offset() {
        let client = test_client();
        let prepared = client.prepare_get(FlightsQuery::list(20, 5)).unwrap();

        assert_eq!(
            prepared.url,
            "https://example.supabase.co/rest/v1/flights?select=*&limit=20&offset=5",
        );
    }

    #[test]
    fn single_flight_request_uses_eq_filter() {
        let client = test_client();
        let prepared = client
            .prepare_get(FlightsQuery::single("flight-1".to_string()))
            .unwrap();

        assert_eq!(
            prepared.url,
            "https://example.supabase.co/rest/v1/flights?select=*&id=eq.flight-1&limit=1&offset=0",
        );
    }
}
