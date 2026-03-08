use std::sync::OnceLock;

use http_body_util::BodyExt;
use worker::*;

use crate::http_client::OriginClient;
use crate::schema::{self, FlightSchema};

static SCHEMA: OnceLock<FlightSchema> = OnceLock::new();

pub fn health() -> Result<http::Response<String>> {
    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(r#"{"status":"ok"}"#.to_string())
        .unwrap())
}

pub async fn graphql(
    req: HttpRequest,
    env: Env,
) -> Result<http::Response<String>> {
    let schema = match SCHEMA.get() {
        Some(s) => s,
        None => {
            let origin_base_url = match env.var("ORIGIN_BASE_URL") {
                Ok(v) => v.to_string(),
                Err(_) => {
                    return Ok(http::Response::builder()
                        .status(502)
                        .header("content-type", "application/json")
                        .body(r#"{"error":"Service misconfigured"}"#.to_string())
                        .unwrap());
                }
            };
            let client = OriginClient::new(origin_base_url);
            SCHEMA.get_or_init(|| schema::build_schema(Box::new(client)))
        }
    };

    let body = req
        .into_body()
        .collect()
        .await
        .map_err(|e| worker::Error::RustError(format!("Failed to read body: {e}")))?
        .to_bytes();

    let gql_request: async_graphql::Request = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            let error_body = serde_json::json!({
                "data": null,
                "errors": [{"message": format!("Invalid request body: {e}")}]
            });
            return Ok(http::Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(serde_json::to_string(&error_body).unwrap())
                .unwrap());
        }
    };

    let gql_response = schema.execute(gql_request).await;
    let response_body = serde_json::to_string(&gql_response).unwrap();

    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(response_body)
        .unwrap())
}
