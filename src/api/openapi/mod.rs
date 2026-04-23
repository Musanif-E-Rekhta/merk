use aide::axum::ApiRouter;
use aide::openapi::{Info, OpenApi, ReferenceOr, SecurityRequirement, SecurityScheme};
use aide::scalar::Scalar;
use axum::Extension;
use axum::Json;
use axum::response::IntoResponse;
use axum::routing::get;
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::Arc;

pub fn router() -> ApiRouter<()> {
    ApiRouter::new()
        .route("/openapi.json", get(serve_openapi))
        .route("/scalar", Scalar::new("/docs/openapi.json").axum_route())
}

pub fn metrics_router(handle: PrometheusHandle) -> ApiRouter<()> {
    ApiRouter::new().route("/metrics", get(move || async move { handle.render() }))
}

pub fn setup() -> OpenApi {
    let mut api = OpenApi {
        info: Info {
            title: "Merk API".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    configure_security(&mut api);
    api
}

fn configure_security(api: &mut OpenApi) {
    let scheme_name = "Bearer";
    let components = api.components.get_or_insert_default();

    components.security_schemes.insert(
        scheme_name.to_string(),
        ReferenceOr::Item(SecurityScheme::Http {
            scheme: scheme_name.to_string(),
            bearer_format: Some("JWT".to_string()),
            extensions: Default::default(),
            description: Some("Bearer <token>".to_string()),
        }),
    );

    let mut requirement = SecurityRequirement::default();
    requirement.insert(scheme_name.to_string(), vec![]);
    api.security.push(requirement);
}

async fn serve_openapi(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoResponse {
    Json(api.as_ref()).into_response()
}
