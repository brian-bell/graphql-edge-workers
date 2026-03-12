use std::cell::RefCell;
use std::future::Future;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use worker::wasm_bindgen::JsCast;
use worker::{Fetch, Url};
#[cfg(target_arch = "wasm32")]
use worker::js_sys;

use crate::config::RuntimeConfig;

const JWKS_CACHE_TTL_SECONDS: u64 = 300;

thread_local! {
    static JWKS_CACHE: RefCell<Option<CachedJwks>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthContext {
    pub access_token: String,
    pub user_id: String,
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    MissingAuthorization,
    MalformedAuthorization,
    InvalidToken(String),
    MissingSigningKey,
    UnsupportedAlgorithm(String),
    JwksFetchFailed(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingAuthorization => f.write_str("Missing Authorization header"),
            AuthError::MalformedAuthorization => f.write_str("Malformed Authorization header"),
            AuthError::InvalidToken(message) => write!(f, "Invalid token: {message}"),
            AuthError::MissingSigningKey => f.write_str("Missing signing key"),
            AuthError::UnsupportedAlgorithm(alg) => {
                write!(f, "Unsupported JWT signing algorithm: {alg}")
            }
            AuthError::JwksFetchFailed(message) => write!(f, "JWKS fetch failed: {message}"),
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Debug, Clone)]
pub struct DecodedJwt {
    pub header: JwtHeader,
    pub claims: JwtClaims,
    pub signing_input: String,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct JwtClaims {
    pub sub: String,
    pub iss: String,
    pub exp: u64,
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Debug, Clone, Deserialize)]
pub struct JwtHeader {
    pub alg: String,
    pub kid: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Jwk {
    alg: Option<String>,
    crv: Option<String>,
    e: Option<String>,
    key_ops: Option<Vec<String>>,
    kid: Option<String>,
    kty: String,
    n: Option<String>,
    #[serde(rename = "use")]
    use_: Option<String>,
    x: Option<String>,
    y: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedJwks {
    fetched_at: u64,
    jwks: Jwks,
}

pub async fn authenticate_headers(
    headers: &http::HeaderMap,
    config: &RuntimeConfig,
) -> Result<AuthContext, AuthError> {
    authenticate_headers_with(headers, config, |decoded, config| async move {
        let jwks = get_jwks(&config).await?;
        verify_signature(&decoded, &jwks).await
    })
    .await
}

pub async fn authenticate_headers_with<F, Fut>(
    headers: &http::HeaderMap,
    config: &RuntimeConfig,
    verifier: F,
) -> Result<AuthContext, AuthError>
where
    F: Fn(DecodedJwt, RuntimeConfig) -> Fut,
    Fut: Future<Output = Result<(), AuthError>>,
{
    let token = extract_bearer_token(headers)?;
    let decoded = decode_jwt(&token)?;
    validate_claims(&decoded.claims, config, now_unix_timestamp())?;
    let user_id = decoded.claims.sub.clone();
    verifier(decoded, config.clone()).await?;

    Ok(AuthContext {
        access_token: token,
        user_id,
    })
}

fn extract_bearer_token(headers: &http::HeaderMap) -> Result<String, AuthError> {
    let header = headers
        .get(http::header::AUTHORIZATION)
        .ok_or(AuthError::MissingAuthorization)?
        .to_str()
        .map_err(|_| AuthError::MalformedAuthorization)?;

    let (scheme, token) = header
        .split_once(' ')
        .ok_or(AuthError::MalformedAuthorization)?;

    if !scheme.eq_ignore_ascii_case("bearer") || token.trim().is_empty() {
        return Err(AuthError::MalformedAuthorization);
    }

    Ok(token.trim().to_string())
}

fn decode_jwt(token: &str) -> Result<DecodedJwt, AuthError> {
    let mut parts = token.split('.');
    let encoded_header = parts
        .next()
        .ok_or_else(|| AuthError::InvalidToken("missing header".to_string()))?;
    let encoded_claims = parts
        .next()
        .ok_or_else(|| AuthError::InvalidToken("missing claims".to_string()))?;
    let encoded_signature = parts
        .next()
        .ok_or_else(|| AuthError::InvalidToken("missing signature".to_string()))?;

    if parts.next().is_some() {
        return Err(AuthError::InvalidToken(
            "expected exactly three JWT segments".to_string(),
        ));
    }

    let header = decode_segment::<JwtHeader>(encoded_header)?;
    let claims = decode_segment::<JwtClaims>(encoded_claims)?;
    let signature = URL_SAFE_NO_PAD
        .decode(encoded_signature)
        .map_err(|e| AuthError::InvalidToken(format!("invalid signature encoding: {e}")))?;

    Ok(DecodedJwt {
        header,
        claims,
        signing_input: format!("{encoded_header}.{encoded_claims}"),
        signature,
    })
}

fn decode_segment<T: DeserializeOwned>(segment: &str) -> Result<T, AuthError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|e| AuthError::InvalidToken(format!("invalid base64url segment: {e}")))?;
    serde_json::from_slice(&decoded)
        .map_err(|e| AuthError::InvalidToken(format!("invalid JWT JSON: {e}")))
}

fn validate_claims(
    claims: &JwtClaims,
    config: &RuntimeConfig,
    now_unix_timestamp: u64,
) -> Result<(), AuthError> {
    if claims.sub.trim().is_empty() {
        return Err(AuthError::InvalidToken("missing subject".to_string()));
    }

    if claims.exp <= now_unix_timestamp {
        return Err(AuthError::InvalidToken("token expired".to_string()));
    }

    if normalize_issuer(&claims.iss) != normalize_issuer(&config.jwt_issuer()) {
        return Err(AuthError::InvalidToken("issuer mismatch".to_string()));
    }

    Ok(())
}

fn normalize_issuer(issuer: &str) -> &str {
    issuer.trim_end_matches('/')
}

fn now_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

async fn get_jwks(config: &RuntimeConfig) -> Result<Jwks, AuthError> {
    let now = now_unix_timestamp();
    if let Some(jwks) = cached_jwks(now) {
        return Ok(jwks);
    }

    let url = Url::parse(&config.jwks_url())
        .map_err(|e| AuthError::JwksFetchFailed(format!("invalid JWKS URL: {e}")))?;
    let mut response = Fetch::Url(url)
        .send()
        .await
        .map_err(|e| AuthError::JwksFetchFailed(format!("request failed: {e}")))?;

    if response.status_code() >= 400 {
        return Err(AuthError::JwksFetchFailed(format!(
            "received HTTP {}",
            response.status_code()
        )));
    }

    let body = response
        .text()
        .await
        .map_err(|e| AuthError::JwksFetchFailed(format!("read failed: {e}")))?;
    let jwks: Jwks = serde_json::from_str(&body)
        .map_err(|e| AuthError::JwksFetchFailed(format!("invalid JWKS JSON: {e}")))?;

    if jwks.keys.is_empty() {
        return Err(AuthError::JwksFetchFailed(
            "JWKS returned no signing keys".to_string(),
        ));
    }

    JWKS_CACHE.with(|cache| {
        *cache.borrow_mut() = Some(CachedJwks {
            fetched_at: now,
            jwks: jwks.clone(),
        });
    });

    Ok(jwks)
}

fn cached_jwks(now_unix_timestamp: u64) -> Option<Jwks> {
    JWKS_CACHE.with(|cache| {
        let cached = cache.borrow();
        cached
            .as_ref()
            .filter(|entry| now_unix_timestamp.saturating_sub(entry.fetched_at) < JWKS_CACHE_TTL_SECONDS)
            .map(|entry| entry.jwks.clone())
    })
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn find_signing_key<'a>(
    decoded: &DecodedJwt,
    jwks: &'a Jwks,
) -> Result<&'a Jwk, AuthError> {
    let key = match decoded.header.kid.as_deref() {
        Some(kid) => jwks.keys.iter().find(|key| key.kid.as_deref() == Some(kid)),
        None if jwks.keys.len() == 1 => jwks.keys.first(),
        None => None,
    }
    .ok_or(AuthError::MissingSigningKey)?;

    if let Some(alg) = &key.alg {
        if alg != &decoded.header.alg {
            return Err(AuthError::InvalidToken("signing key algorithm mismatch".to_string()));
        }
    }

    if let Some(use_) = &key.use_ {
        if use_ != "sig" {
            return Err(AuthError::InvalidToken("signing key cannot verify signatures".to_string()));
        }
    }

    if let Some(key_ops) = &key.key_ops {
        if !key_ops.iter().any(|op| op == "verify") {
            return Err(AuthError::InvalidToken("signing key missing verify operation".to_string()));
        }
    }

    Ok(key)
}

#[cfg(target_arch = "wasm32")]
async fn verify_signature(decoded: &DecodedJwt, jwks: &Jwks) -> Result<(), AuthError> {
    match decoded.header.alg.as_str() {
        "RS256" => verify_rs256(decoded, find_signing_key(decoded, jwks)?).await,
        "ES256" => verify_es256(decoded, find_signing_key(decoded, jwks)?).await,
        other => Err(AuthError::UnsupportedAlgorithm(other.to_string())),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn verify_signature(decoded: &DecodedJwt, jwks: &Jwks) -> Result<(), AuthError> {
    let _ = (decoded, jwks);
    Err(AuthError::InvalidToken(
        "signature verification requires the wasm32 target".to_string(),
    ))
}

#[cfg(target_arch = "wasm32")]
async fn verify_rs256(decoded: &DecodedJwt, jwk: &Jwk) -> Result<(), AuthError> {
    if jwk.kty != "RSA" {
        return Err(AuthError::InvalidToken("expected RSA signing key".to_string()));
    }
    verify_with_webcrypto(
        jwk,
        import_algorithm("RSASSA-PKCS1-v1_5", Some("SHA-256"), None),
        "RSASSA-PKCS1-v1_5",
        decoded.signature.as_slice(),
        decoded.signing_input.as_bytes(),
    )
    .await
}

#[cfg(target_arch = "wasm32")]
async fn verify_es256(decoded: &DecodedJwt, jwk: &Jwk) -> Result<(), AuthError> {
    if jwk.kty != "EC" {
        return Err(AuthError::InvalidToken("expected EC signing key".to_string()));
    }
    if jwk.crv.as_deref() != Some("P-256") {
        return Err(AuthError::InvalidToken("expected P-256 signing key".to_string()));
    }

    let der_signature = jose_es256_signature_to_der(&decoded.signature)?;

    verify_with_webcrypto(
        jwk,
        import_algorithm("ECDSA", None, Some("P-256")),
        verify_algorithm("ECDSA", "SHA-256"),
        der_signature.as_slice(),
        decoded.signing_input.as_bytes(),
    )
    .await
}

#[cfg(target_arch = "wasm32")]
async fn verify_with_webcrypto(
    jwk: &Jwk,
    import_algorithm: js_sys::Object,
    verify_algorithm: impl Into<VerifyAlgorithm>,
    signature: &[u8],
    data: &[u8],
) -> Result<(), AuthError> {
    use wasm_bindgen_futures::JsFuture;

    let global: web_sys::WorkerGlobalScope = js_sys::global().unchecked_into();
    let crypto = global
        .crypto()
        .map_err(|_| AuthError::InvalidToken("Web Crypto unavailable".to_string()))?;
    let subtle = crypto.subtle();
    let key_data = serde_wasm_bindgen::to_value(jwk)
        .map_err(|e| AuthError::InvalidToken(format!("invalid JWK value: {e}")))?;
    let key_data = key_data
        .dyn_into::<js_sys::Object>()
        .map_err(|_| AuthError::InvalidToken("invalid JWK object".to_string()))?;
    let usages = js_sys::Array::new();
    usages.push(&"verify".into());

    let imported = JsFuture::from(
        subtle
            .import_key_with_object("jwk", &key_data, &import_algorithm, false, &usages.into())
            .map_err(|e| AuthError::InvalidToken(js_error_message(e)))?,
    )
    .await
    .map_err(|e| AuthError::InvalidToken(js_error_message(e)))?;
    let crypto_key = imported
        .dyn_into::<web_sys::CryptoKey>()
        .map_err(|_| AuthError::InvalidToken("failed to import crypto key".to_string()))?;

    let verified = match verify_algorithm.into() {
        VerifyAlgorithm::Name(name) => JsFuture::from(
            subtle
                .verify_with_str_and_u8_array_and_u8_array(
                    &name,
                    &crypto_key,
                    signature,
                    data,
                )
                .map_err(|e| AuthError::InvalidToken(js_error_message(e)))?,
        )
        .await
        .map_err(|e| AuthError::InvalidToken(js_error_message(e)))?,
        VerifyAlgorithm::Object(algorithm) => JsFuture::from(
            subtle
                .verify_with_object_and_u8_array_and_u8_slice(
                    &algorithm,
                    &crypto_key,
                    &js_sys::Uint8Array::from(signature),
                    data,
                )
                .map_err(|e| AuthError::InvalidToken(js_error_message(e)))?,
        )
        .await
        .map_err(|e| AuthError::InvalidToken(js_error_message(e)))?,
    };

    if verified.as_bool() == Some(true) {
        Ok(())
    } else {
        Err(AuthError::InvalidToken("signature verification failed".to_string()))
    }
}

#[cfg(target_arch = "wasm32")]
enum VerifyAlgorithm {
    Name(String),
    Object(js_sys::Object),
}

#[cfg(target_arch = "wasm32")]
impl From<&str> for VerifyAlgorithm {
    fn from(value: &str) -> Self {
        VerifyAlgorithm::Name(value.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
impl From<js_sys::Object> for VerifyAlgorithm {
    fn from(value: js_sys::Object) -> Self {
        VerifyAlgorithm::Object(value)
    }
}

#[cfg(target_arch = "wasm32")]
fn import_algorithm(
    name: &str,
    hash_name: Option<&str>,
    named_curve: Option<&str>,
) -> js_sys::Object {
    let algorithm = js_sys::Object::new();
    set_js_prop(&algorithm, "name", &name.into());

    if let Some(hash_name) = hash_name {
        let hash = js_sys::Object::new();
        set_js_prop(&hash, "name", &hash_name.into());
        set_js_prop(&algorithm, "hash", &hash.into());
    }

    if let Some(named_curve) = named_curve {
        set_js_prop(&algorithm, "namedCurve", &named_curve.into());
    }

    algorithm
}

#[cfg(target_arch = "wasm32")]
fn verify_algorithm(name: &str, hash_name: &str) -> js_sys::Object {
    let algorithm = js_sys::Object::new();
    set_js_prop(&algorithm, "name", &name.into());

    let hash = js_sys::Object::new();
    set_js_prop(&hash, "name", &hash_name.into());
    set_js_prop(&algorithm, "hash", &hash.into());

    algorithm
}

#[cfg(target_arch = "wasm32")]
fn set_js_prop(object: &js_sys::Object, key: &str, value: &worker::wasm_bindgen::JsValue) {
    let _ = js_sys::Reflect::set(object, &key.into(), value);
}

#[cfg(target_arch = "wasm32")]
fn js_error_message(error: worker::wasm_bindgen::JsValue) -> String {
    error
        .as_string()
        .unwrap_or_else(|| "JavaScript error".to_string())
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn jose_es256_signature_to_der(signature: &[u8]) -> Result<Vec<u8>, AuthError> {
    if signature.len() != 64 {
        return Err(AuthError::InvalidToken(
            "invalid ES256 signature length".to_string(),
        ));
    }

    let (r, s) = signature.split_at(32);
    let der_r = der_integer(r);
    let der_s = der_integer(s);
    let content_len = der_r.len() + der_s.len();

    let mut der = Vec::with_capacity(content_len + 2);
    der.push(0x30);
    der.push(content_len as u8);
    der.extend_from_slice(&der_r);
    der.extend_from_slice(&der_s);

    Ok(der)
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn der_integer(bytes: &[u8]) -> Vec<u8> {
    let mut trimmed = bytes
        .iter()
        .skip_while(|byte| **byte == 0)
        .copied()
        .collect::<Vec<_>>();

    if trimmed.is_empty() {
        trimmed.push(0);
    }

    if trimmed[0] & 0x80 != 0 {
        trimmed.insert(0, 0);
    }

    let mut encoded = Vec::with_capacity(trimmed.len() + 2);
    encoded.push(0x02);
    encoded.push(trimmed.len() as u8);
    encoded.extend_from_slice(&trimmed);
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuntimeConfig;

    fn make_token(
        claims: serde_json::Value,
        header: serde_json::Value,
    ) -> String {
        let encode = |value: serde_json::Value| {
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&value).unwrap())
        };
        let signature = URL_SAFE_NO_PAD.encode([1_u8, 2, 3, 4]);

        format!("{}.{}.{}", encode(header), encode(claims), signature)
    }

    fn test_config() -> RuntimeConfig {
        RuntimeConfig::new(
            "https://example.supabase.co",
            "publishable-key",
        )
    }

    #[tokio::test]
    async fn extracts_valid_bearer_token_and_subject() {
        let claims = serde_json::json!({
            "sub": "user-123",
            "iss": "https://example.supabase.co/auth/v1",
            "exp": now_unix_timestamp() + 60,
        });
        let header = serde_json::json!({
            "alg": "RS256",
            "kid": "test-key",
        });
        let token = make_token(claims, header);

        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::AUTHORIZATION,
            format!("Bearer {token}").parse().unwrap(),
        );

        let auth = authenticate_headers_with(&headers, &test_config(), |_, _| async { Ok(()) })
            .await
            .unwrap();

        assert_eq!(auth.user_id, "user-123");
        assert_eq!(auth.access_token, token);
    }

    #[tokio::test]
    async fn rejects_missing_authorization_header() {
        let err = authenticate_headers_with(
            &http::HeaderMap::new(),
            &test_config(),
            |_, _| async { Ok(()) },
        )
        .await
        .unwrap_err();

        assert_eq!(err, AuthError::MissingAuthorization);
    }

    #[tokio::test]
    async fn rejects_malformed_authorization_header() {
        let mut headers = http::HeaderMap::new();
        headers.insert(http::header::AUTHORIZATION, "NotBearer".parse().unwrap());

        let err = authenticate_headers_with(&headers, &test_config(), |_, _| async { Ok(()) })
            .await
            .unwrap_err();

        assert_eq!(err, AuthError::MalformedAuthorization);
    }

    #[tokio::test]
    async fn rejects_expired_token() {
        let token = make_token(
            serde_json::json!({
                "sub": "user-123",
                "iss": "https://example.supabase.co/auth/v1",
                "exp": now_unix_timestamp() - 1,
            }),
            serde_json::json!({
                "alg": "RS256",
                "kid": "test-key",
            }),
        );
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::AUTHORIZATION,
            format!("Bearer {token}").parse().unwrap(),
        );

        let err = authenticate_headers_with(&headers, &test_config(), |_, _| async { Ok(()) })
            .await
            .unwrap_err();

        assert_eq!(
            err,
            AuthError::InvalidToken("token expired".to_string()),
        );
    }

    #[tokio::test]
    async fn rejects_invalid_signature_from_verifier() {
        let token = make_token(
            serde_json::json!({
                "sub": "user-123",
                "iss": "https://example.supabase.co/auth/v1",
                "exp": now_unix_timestamp() + 60,
            }),
            serde_json::json!({
                "alg": "RS256",
                "kid": "test-key",
            }),
        );
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::AUTHORIZATION,
            format!("Bearer {token}").parse().unwrap(),
        );

        let err = authenticate_headers_with(&headers, &test_config(), |_, _| async {
            Err(AuthError::InvalidToken(
                "signature verification failed".to_string(),
            ))
        })
        .await
        .unwrap_err();

        assert_eq!(
            err,
            AuthError::InvalidToken("signature verification failed".to_string()),
        );
    }

    #[test]
    fn converts_jose_es256_signature_to_der() {
        let mut jose = vec![0; 64];
        jose[31] = 1;
        jose[63] = 2;

        let der = jose_es256_signature_to_der(&jose).unwrap();

        assert_eq!(der, vec![0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x02]);
    }
}
