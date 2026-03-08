use std::future::Future;
use std::pin::Pin;

use serde::de::DeserializeOwned;
use worker::{Fetch, Url};

use crate::models::{CreateFlightInput, Flight};

pub trait FlightApi: Send + Sync {
    fn get_flight(&self, id: String) -> Pin<Box<dyn Future<Output = Result<Flight, String>> + '_>>;
    fn get_flights(
        &self,
        limit: i32,
        offset: i32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Flight>, String>> + '_>>;
    fn create_flight(
        &self,
        input: CreateFlightInput,
    ) -> Pin<Box<dyn Future<Output = Result<Flight, String>> + '_>>;
}

pub struct OriginClient {
    base_url: String,
}

impl OriginClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}{}", self.base_url, path);
        let parsed_url = Url::parse(&url).map_err(|e| format!("Invalid URL: {e}"))?;

        let mut response = Fetch::Url(parsed_url)
            .send()
            .await
            .map_err(|e| format!("Fetch failed: {e}"))?;

        if response.status_code() >= 400 {
            return Err(format!("Origin returned status {}", response.status_code()));
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {e}"))?;

        serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse response JSON: {e}"))
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let url = format!("{}{}", self.base_url, path);
        Url::parse(&url).map_err(|e| format!("Invalid URL: {e}"))?;

        let body_json = serde_json::to_string(body)
            .map_err(|e| format!("Failed to serialize request body: {e}"))?;

        let mut request_init = worker::RequestInit::new();
        request_init.with_method(worker::Method::Post);
        request_init
            .headers
            .set("Content-Type", "application/json")
            .map_err(|e| format!("Failed to set Content-Type header: {e}"))?;
        request_init.with_body(Some(worker::wasm_bindgen::JsValue::from_str(&body_json)));

        let request = worker::Request::new_with_init(&url, &request_init)
            .map_err(|e| format!("Failed to create request: {e}"))?;

        let mut response = Fetch::Request(request)
            .send()
            .await
            .map_err(|e| format!("Fetch failed: {e}"))?;

        if response.status_code() >= 400 {
            return Err(format!("Origin returned status {}", response.status_code()));
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {e}"))?;

        serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse response JSON: {e}"))
    }
}

impl FlightApi for OriginClient {
    fn get_flight(&self, id: String) -> Pin<Box<dyn Future<Output = Result<Flight, String>> + '_>> {
        Box::pin(async move {
            let path = format!("/flights/{id}");
            self.get::<Flight>(&path).await
        })
    }

    fn get_flights(
        &self,
        limit: i32,
        offset: i32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Flight>, String>> + '_>> {
        Box::pin(async move {
            let path = format!("/flights?limit={limit}&offset={offset}");
            self.get::<Vec<Flight>>(&path).await
        })
    }

    fn create_flight(
        &self,
        input: CreateFlightInput,
    ) -> Pin<Box<dyn Future<Output = Result<Flight, String>> + '_>> {
        Box::pin(async move { self.post::<Flight, _>("/flights", &input).await })
    }
}
