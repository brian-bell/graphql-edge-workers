use std::sync::OnceLock;

use http_body_util::{BodyExt, Limited};
use worker::*;

use crate::auth::{self, AuthContext, AuthError};
use crate::config::RuntimeConfig;
use crate::http_client::{FlightApi, SupabaseClient};
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
    let config = match RuntimeConfig::from_env(&env) {
        Ok(config) => config,
        Err(_) => return Ok(service_misconfigured_response()),
    };

    let auth = match require_auth(auth::authenticate_headers(req.headers(), &config).await) {
        Ok(auth) => auth,
        Err(response) => return Ok(response),
    };

    graphql_with_auth(req, config, auth).await
}

async fn graphql_with_auth(
    req: HttpRequest,
    config: RuntimeConfig,
    auth: AuthContext,
) -> Result<http::Response<String>> {
    let schema = SCHEMA.get_or_init(schema::build_base_schema);

    if let Some(resp) = reject_oversized_body(req.headers()) {
        return Ok(resp);
    }

    let body = match Limited::new(req.into_body(), MAX_BODY_SIZE as usize)
        .collect()
        .await
    {
        Ok(collected) => collected.to_bytes(),
        Err(_) => {
            return Ok(payload_too_large_response());
        }
    };

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

    let client: Box<dyn FlightApi> = Box::new(SupabaseClient::new(config, auth));
    let gql_response = schema.execute(gql_request.data(client)).await;
    let response_body = serde_json::to_string(&gql_response).unwrap();

    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(response_body)
        .unwrap())
}

const MAX_BODY_SIZE: u64 = 8_192;

fn reject_oversized_body(headers: &http::HeaderMap) -> Option<http::Response<String>> {
    let len = headers
        .get("content-length")?
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()?;
    if len > MAX_BODY_SIZE {
        Some(payload_too_large_response())
    } else {
        None
    }
}

fn payload_too_large_response() -> http::Response<String> {
    http::Response::builder()
        .status(413)
        .header("content-type", "application/json")
        .body(r#"{"error":"Request body too large"}"#.to_string())
        .unwrap()
}

fn require_auth(
    auth: std::result::Result<AuthContext, AuthError>,
) -> std::result::Result<AuthContext, http::Response<String>> {
    auth.map_err(|_| unauthorized_response())
}

fn unauthorized_response() -> http::Response<String> {
    http::Response::builder()
        .status(401)
        .header("content-type", "application/json")
        .body(r#"{"error":"Unauthorized"}"#.to_string())
        .unwrap()
}

fn service_misconfigured_response() -> http::Response<String> {
    http::Response::builder()
        .status(502)
        .header("content-type", "application/json")
        .body(r#"{"error":"Service misconfigured"}"#.to_string())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthError;

    #[test]
    fn rejects_body_over_8kb() {
        let mut headers = http::HeaderMap::new();
        headers.insert("content-length", "10000".parse().unwrap());
        let resp = reject_oversized_body(&headers).expect("should reject");
        assert_eq!(resp.status(), 413);
    }

    #[test]
    fn allows_body_at_8kb() {
        let mut headers = http::HeaderMap::new();
        headers.insert("content-length", "8192".parse().unwrap());
        assert!(reject_oversized_body(&headers).is_none());
    }

    #[test]
    fn allows_body_under_8kb() {
        let mut headers = http::HeaderMap::new();
        headers.insert("content-length", "100".parse().unwrap());
        assert!(reject_oversized_body(&headers).is_none());
    }

    #[test]
    fn allows_missing_content_length() {
        let headers = http::HeaderMap::new();
        assert!(reject_oversized_body(&headers).is_none());
    }

    #[test]
    fn allows_invalid_content_length() {
        let mut headers = http::HeaderMap::new();
        headers.insert("content-length", "not-a-number".parse().unwrap());
        assert!(reject_oversized_body(&headers).is_none());
    }

    #[test]
    fn graphql_requires_authorization_header() {
        let response = require_auth(Err(AuthError::MissingAuthorization)).unwrap_err();
        assert_eq!(response.status(), 401);
    }

    #[test]
    fn graphql_rejects_invalid_authorization_header() {
        let response = require_auth(Err(AuthError::InvalidToken(
            "signature verification failed".to_string(),
        )))
        .unwrap_err();
        assert_eq!(response.status(), 401);
    }
}
