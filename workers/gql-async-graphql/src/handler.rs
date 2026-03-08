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

    if let Some(resp) = reject_oversized_body(req.headers()) {
        return Ok(resp);
    }

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

const MAX_BODY_SIZE: u64 = 8_192; // 8 KB

fn reject_oversized_body(headers: &http::HeaderMap) -> Option<http::Response<String>> {
    let len = headers
        .get("content-length")?
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()?;
    if len > MAX_BODY_SIZE {
        Some(
            http::Response::builder()
                .status(413)
                .header("content-type", "application/json")
                .body(r#"{"error":"Request body too large"}"#.to_string())
                .unwrap(),
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
