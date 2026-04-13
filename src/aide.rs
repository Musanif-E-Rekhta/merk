use aide::axum::ApiRouter;
use aide::axum::routing::get;
use aide::openapi::OpenApi;
use axum::{Extension, Json};
use axum::response::{Html, IntoResponse};
use std::sync::Arc;

pub async fn serve_openapi(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoResponse {
    Json(api.as_ref()).into_response()
}

pub async fn serve_scalar() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html>
  <head>
    <title>API Reference</title>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
  </head>
  <body>
    <script
      id="api-reference"
      data-url="/docs/openapi.json"
    ></script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
  </body>
</html>
"#)
}

pub fn docs_routes() -> ApiRouter {
    ApiRouter::new()
        .route("/openapi.json", get(serve_openapi))
        .route("/scalar", get(serve_scalar))
}
