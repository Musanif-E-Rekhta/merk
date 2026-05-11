//! Single-binary deploy: serve the musanif web build from `frontend-dist/`.
//!
//! Wired only when the `embed-frontend` feature is enabled (default off so
//! local dev doesn't need the dist directory populated). The Dockerfile's
//! `frontend` stage runs `dx build --release` then copies the wasm bundle
//! into `merk/frontend-dist/` so the runtime build embeds it.
//!
//! Routing: anything not matched by `/api/*` or `/docs` falls through to
//! this module. Unknown paths inside the SPA fall back to `index.html`
//! so client-side router routes (e.g. `/books/sarab`) work after a
//! browser refresh.

use aide::axum::ApiRouter;
use axum::{
    body::Body,
    http::{HeaderValue, Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::any,
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "frontend-dist/"]
struct Frontend;

pub fn routes() -> ApiRouter<()> {
    ApiRouter::new().route("/{*path}", any(serve)).route("/", any(index))
}

async fn index() -> Response {
    serve_path("index.html")
}

async fn serve(req: Request<Body>) -> Response {
    let path = req.uri().path().trim_start_matches('/');
    let resp = serve_path(path);
    if resp.status() == StatusCode::NOT_FOUND {
        // SPA fallback for client-side routes.
        serve_path("index.html")
    } else {
        resp
    }
}

fn serve_path(path: &str) -> Response {
    match Frontend::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut resp = Response::new(Body::from(file.data.into_owned()));
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref())
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            resp
        }
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}
