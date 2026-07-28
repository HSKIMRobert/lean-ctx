use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

#[derive(Clone, Debug)]
pub(crate) struct DetectedWebApp(pub(crate) super::web_app::WebAppProvider);

pub(crate) async fn detect_web_app(request: Request, next: Next) -> Response {
    let detected = request
        .headers()
        .get("host")
        .and_then(|header| header.to_str().ok())
        .and_then(|host| super::web_app::detect_web_provider(host, request.uri().path()));

    if let Some(provider) = detected {
        let mut request = request;
        request.extensions_mut().insert(DetectedWebApp(provider));
        next.run(request).await
    } else {
        next.run(request).await
    }
}
