use crate::info;
pub use prometheus::default_registry;
pub use prometheus::Registry;
use prometheus::TextEncoder;
use warp;
use warp::Filter;
use warp::Rejection;
use warp::Reply;

async fn metrics_handler() -> Result<impl Reply, Rejection> {
    let encoder = TextEncoder::new();
    let mut buffer = String::from("");

    encoder
        .encode_utf8(&prometheus::gather(), &mut buffer)
        .expect("Failed to encode metrics");

    let response = buffer.clone();
    buffer.clear();

    Ok(response)
}

pub async fn run_metric_server(port: u16) {
    info!(Prometheus, format!("Start metrics server at port: {port}"));
    let metrics_route = warp::path!("metrics").and_then(metrics_handler);
    warp::serve(metrics_route).run(([0, 0, 0, 0], port)).await;
}
