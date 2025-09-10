use tower_http::cors::{CorsLayer, Any};
use hyper::Method;

pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
        .allow_origin(Any)
}
