use std::{error::Error, net::SocketAddr};

use axum::{extract::Host, http::{HeaderMap, Request, Response, StatusCode}, response::IntoResponse, routing::*, Router};
use miette::IntoDiagnostic;

const PROXY_ORGIN_URI : &'static str = "localhost:3000";
const PROXY_FROM_DOMAIN : &'static str = "client.hello";
#[tokio::main]
async fn main() -> Result<(), &'static str> {
    let app = Router::new().fallback(proxy_handler);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    println!("server listening on {}",addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.into_diagnostic().map_err(|_| "error".to_string());
    Ok(())
}

async fn proxy_handler<Body>( host : Host, request : Request<Body>) -> Result<impl IntoResponse, String> {
    let uri = request.uri();
    let split : Vec<_>= host.0.split(':').collect();
    let host_name = split[0];
    println!("host :{}", host_name);
    if host_name != PROXY_FROM_DOMAIN {
        return Err(format!("expected host {} but found {:#?}", PROXY_FROM_DOMAIN.to_string(), host));
    }
    let path = uri.path_and_query().map(|pq| pq.path()).unwrap_or("/");
    let client = reqwest::Client::new();
    let response = client.get(format!("http://{}{}", PROXY_ORGIN_URI, path))
        .send()
        .await
        .map_err(|_| "request failed").unwrap();

    let  status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.into_diagnostic().map_err(|_| "could not find the origin domain");
    
    let mut axum_response = Response::new(axum::body::Body::from(body?));
    *axum_response.status_mut() = status;
    let axum_headers = axum_response.headers_mut();
    axum_headers.extend(headers.iter().map(|(name, value)| (name.clone(), value.clone())));

    Ok(axum_response)
}