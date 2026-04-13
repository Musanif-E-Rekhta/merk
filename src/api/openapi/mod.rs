use aide::axum::ApiRouter;
use aide::openapi::{Info, OpenApi, ReferenceOr, SecurityRequirement, SecurityScheme};
use aide::scalar::Scalar;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Json};
use std::sync::Arc;

pub async fn serve_openapi(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoResponse {
    Json(api.as_ref()).into_response()
}

pub fn docs_routes() -> ApiRouter<()> {
    ApiRouter::new()
        .route("/openapi.json", get(serve_openapi))
        .route("/scalar", Scalar::new("/docs/openapi.json").axum_route())
}

pub fn setup_aide() -> OpenApi {
    let mut api = OpenApi {
        info: Info {
            title: "Merk API".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    configure_openapi(&mut api);
    api
}

pub fn configure_openapi(api: &mut OpenApi) {
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
