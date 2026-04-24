use aide::axum::ApiRouter;
use axum::routing::get;
use metrics_exporter_prometheus::PrometheusBuilder;

pub fn router() -> ApiRouter<()> {
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    ApiRouter::new().route("/metrics", get(move || async move { handle.render() }))
}
