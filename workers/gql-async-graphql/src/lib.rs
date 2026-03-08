use worker::*;

mod handler;
mod schema;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<http::Response<String>> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    match (method, path.as_str()) {
        (http::Method::GET, "/health") => handler::health(),
        (http::Method::POST, "/graphql") => handler::graphql(req, env).await,
        _ => Ok(http::Response::builder()
            .status(404)
            .body("Not Found".to_string())
            .unwrap()),
    }
}
