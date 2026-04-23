use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use metrics::{counter, gauge, histogram};
use std::time::Instant;

pub async fn track_metrics(req: Request, next: Next) -> Response {
    if req.uri().path() == "/metrics" {
        return next.run(req).await;
    }

    let method = req.method().to_string();

    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|mp: &MatchedPath| mp.as_str().to_owned())
        .unwrap_or_else(|| req.uri().path().to_owned());

    gauge!("axum_http_requests_pending", "method" => method.clone(), "path" => path.clone())
        .increment(1.0);

    let start = Instant::now();
    let response = next.run(req).await;
    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    gauge!("axum_http_requests_pending", "method" => method.clone(), "path" => path.clone())
        .decrement(1.0);

    counter!("axum_http_requests_total", "method" => method.clone(), "path" => path.clone(), "status" => status)
        .increment(1);

    histogram!("axum_http_requests_duration_seconds", "method" => method, "path" => path)
        .record(latency);

    response
}
