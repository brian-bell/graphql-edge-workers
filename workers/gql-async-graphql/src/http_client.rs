use std::fmt;
use std::future::Future;
use std::pin::Pin;

use serde::de::DeserializeOwned;
use worker::{Fetch, Url};

use crate::models::{CreateFlightInput, Flight};

#[derive(Debug)]
pub enum OriginError {
    /// HTTP error response from the origin API.
    Status(u16),
    /// Network, parsing, or other non-HTTP errors.
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
            OriginError::Other(msg) => f.write_str(msg),
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
}

pub struct OriginClient {
    base_url: String,
}

impl OriginClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, OriginError> {
        let url = format!("{}{}", self.base_url, path);
        let parsed_url =
            Url::parse(&url).map_err(|e| OriginError::Other(format!("Invalid URL: {e}")))?;

        let mut response = Fetch::Url(parsed_url)
            .send()
            .await
            .map_err(|e| OriginError::Other(format!("Fetch failed: {e}")))?;

        if response.status_code() >= 400 {
            return Err(OriginError::Status(response.status_code()));
        }

        let text = response
            .text()
            .await
            .map_err(|e| OriginError::Other(format!("Failed to read response body: {e}")))?;

        serde_json::from_str(&text)
            .map_err(|e| OriginError::Other(format!("Failed to parse response JSON: {e}")))
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, OriginError> {
        let url = format!("{}{}", self.base_url, path);
        let parsed_url =
            Url::parse(&url).map_err(|e| OriginError::Other(format!("Invalid URL: {e}")))?;

        let body_json = serde_json::to_string(body)
            .map_err(|e| OriginError::Other(format!("Failed to serialize request body: {e}")))?;

        let mut request_init = worker::RequestInit::new();
        request_init.with_method(worker::Method::Post);
        request_init
            .headers
            .set("Content-Type", "application/json")
            .map_err(|e| {
                OriginError::Other(format!("Failed to set Content-Type header: {e}"))
            })?;
        request_init.with_body(Some(worker::wasm_bindgen::JsValue::from_str(&body_json)));

        let request = worker::Request::new_with_init(parsed_url.as_str(), &request_init)
            .map_err(|e| OriginError::Other(format!("Failed to create request: {e}")))?;

        let mut response = Fetch::Request(request)
            .send()
            .await
            .map_err(|e| OriginError::Other(format!("Fetch failed: {e}")))?;

        if response.status_code() >= 400 {
            return Err(OriginError::Status(response.status_code()));
        }

        let text = response
            .text()
            .await
            .map_err(|e| OriginError::Other(format!("Failed to read response body: {e}")))?;

        serde_json::from_str(&text)
            .map_err(|e| OriginError::Other(format!("Failed to parse response JSON: {e}")))
    }
}

impl FlightApi for OriginClient {
    fn get_flight(
        &self,
        id: String,
    ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>> {
        Box::pin(async move {
            let path = format!("/flights/{id}");
            self.get::<Flight>(&path).await
        })
    }

    fn get_flights(
        &self,
        limit: i32,
        offset: i32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Flight>, OriginError>> + '_>> {
        Box::pin(async move {
            let path = format!("/flights?limit={limit}&offset={offset}");
            self.get::<Vec<Flight>>(&path).await
        })
    }

    fn create_flight(
        &self,
        input: CreateFlightInput,
    ) -> Pin<Box<dyn Future<Output = Result<Flight, OriginError>> + '_>> {
        Box::pin(async move { self.post::<Flight, _>("/flights", &input).await })
    }
}
