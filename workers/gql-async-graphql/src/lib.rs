use worker::*;

#[event(fetch)]
async fn fetch(
    _req: HttpRequest,
    _env: Env,
    _ctx: Context,
) -> Result<http::Response<String>> {
    let response = http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(r#"{"ok": true}"#.to_string())
        .unwrap();
    Ok(response)
}
