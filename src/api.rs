///! Simple HTTP API server to expose tunnel information
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde_json::json;
use tokio::sync::RwLock;
use tracing::info;

/// Tunnel information exposed via API
#[derive(Clone)]
pub struct TunnelInfo {
    pub server_addr: String,
    pub remote_port: u16,
}

/// Start a simple HTTP API server that exposes tunnel information
pub async fn start_api_server(
    api_port: u16,
    tunnel_info: Arc<RwLock<Option<TunnelInfo>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], api_port));

    let make_svc = make_service_fn(move |_conn| {
        let tunnel_info = Arc::clone(&tunnel_info);
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_request(req, Arc::clone(&tunnel_info))
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    info!("API server listening on http://{addr}");
    
    server.await?;
    Ok(())
}

async fn handle_request(
    req: Request<Body>,
    tunnel_info: Arc<RwLock<Option<TunnelInfo>>>,
) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/api/tunnel") => {
            let info = tunnel_info.read().await;
            if let Some(tunnel) = info.as_ref() {
                let response_json = json!({
                    "status": "connected",
                    "server": tunnel.server_addr.clone(),
                    "remote_port": tunnel.remote_port,
                    "public_url": format!("{}:{}", tunnel.server_addr, tunnel.remote_port),
                });
                Ok(Response::new(Body::from(response_json.to_string())))
            } else {
                let response_json = json!({
                    "status": "not_connected",
                    "error": "Tunnel not established yet"
                });
                Ok(Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Body::from(response_json.to_string()))
                    .unwrap())
            }
        }
        (&Method::GET, "/health") => {
            Ok(Response::new(Body::from(json!({"status": "ok"}).to_string())))
        }
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap()),
    }
}

