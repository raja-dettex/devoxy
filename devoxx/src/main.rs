use core::panic;
use std::{borrow::Borrow, clone, collections::HashMap, error::Error, hash::Hash, net::SocketAddr, sync::{Arc, Mutex}};

use axum::{body::{self, Body, HttpBody,  Bytes}, extract::Host, http::{method, uri::{self, PathAndQuery}, HeaderMap, Request, Response, StatusCode, Uri}, response::IntoResponse, routing::*, RequestExt, Router};
use miette::IntoDiagnostic;
use reqwest::Method;
use lazy_static::lazy_static;

#[derive(PartialEq, Eq, Clone)]
struct CacheKey(Method,Uri);

#[derive(Clone, Debug)]
struct CachedResponse {
    status: StatusCode,
    headers: HeaderMap,
    body : Bytes
}

impl CachedResponse  {
    fn default() -> Self {
        CachedResponse{status: StatusCode::OK, headers: HeaderMap::new(), body: Bytes::new()}
    }
    fn new(status : StatusCode, headers : HeaderMap, body: Bytes) -> Self {
        CachedResponse{ status , headers: headers.clone(), body : body}
    }
    fn get_parts(&self) -> (StatusCode, HeaderMap, Bytes) {
        (self.status, self.headers.clone(), self.body.clone())
    }
}

struct Cache {
    inner: Mutex<HashMap<CacheKey, Arc<CachedResponse>>>
}

struct AppState {
    host: Host, 
    method: Method, 
    uri : Uri,
    headers : Arc<Mutex<HeaderMap>>
}


impl Hash for CacheKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.hash(state);
    }
}


impl CacheKey {
    fn new(method : Method, uri : Uri) -> Self{
        CacheKey(method, uri)
        
    }
}
lazy_static! {
    static ref CACHE : Arc<Cache> = Arc::new(Cache {
        inner: Mutex::new(HashMap::new())
    });
}

fn insert_into_cache(method: Method, uri: Uri, status: StatusCode, headers : HeaderMap, body : Bytes) {
    let fresh_cache_key = CacheKey::new(method.clone(), uri.clone());
    let cache_obj = CachedResponse::new(status, headers.clone(), body);
    CACHE.clone().inner.lock().unwrap().insert(fresh_cache_key, Arc::new(cache_obj));
}

fn get_from_cache(method: Method, uri: Uri) -> Arc<CachedResponse>{
    let fresh_cache_key = CacheKey::new(method.clone(), uri.clone());
    let cache = CACHE.inner.lock().unwrap();
    let response = cache.get(&fresh_cache_key).unwrap();
    response.clone()
}

fn is_cached(method: Method, uri: Uri) -> bool{
    let fresh_cache_key = CacheKey::new(method.clone(), uri.clone());
    let exists = CACHE.inner.lock().unwrap().contains_key(&fresh_cache_key);
    exists
}



const PROXY_ORGIN_URI : &'static str = "localhost:3000";
const PROXY_FROM_DOMAIN : &'static str = "client.hello";
#[tokio::main]
async fn main() -> Result<(), &'static str> {
    let app = Router::new().fallback(|request: Request<Body>| async {
        let response = proxy_handler(request)
        .await
        .map_err(|_| "error").unwrap();
        response
    });
    
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    println!("server listening on {}",addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.into_diagnostic().map_err(|_| "error".to_string());
    Ok(())
}

async fn proxy_handler( mut request: Request<Body>) -> Result<Response<Body>, String> {
    let uri : Uri = request.extract_parts().await.unwrap(); 
    let method : Method = request.extract_parts().await.unwrap(); 
    let host: Host = request.extract_parts().await.unwrap();
    let req_headers: HeaderMap = request.extract_parts().await.unwrap();
    
    let split : Vec<_>= host.0.split(':').collect(); 
    let host_name = split[0];
    //println!("host :{}", host_name);
    if host_name != PROXY_FROM_DOMAIN {
        return Err(format!("expected host {} but found {:#?}", PROXY_FROM_DOMAIN.to_string(), host));
    }
    //let path = uri.path_and_query().cloned().map(|pq| pq.path()).unwrap_or("/");
    let p_and_q = uri.path_and_query().cloned().unwrap_or_else(|| PathAndQuery::from_static("/"));
    let url  = uri::Builder::new().scheme("http")
        .authority(PROXY_ORGIN_URI)
        .path_and_query(p_and_q.clone())
        .build()
        .map_err(|_| "could not build url")?;
    let axum_response = get_cached_response(method, url, req_headers).await.map_err(|_| "failed to get cached response")?;
    Ok(axum_response)
}



async fn get_cached_response( method : Method, url: Uri, req_headers : HeaderMap ) -> Result<Response<Body>, String> {
    // todo 1.check the cache, if the response is in the cache return here

        if is_cached(method.clone(), url.clone()) {
            let mut cached_response = get_from_cache(method.clone(), url.clone());
            let mut cached = Arc::make_mut(&mut cached_response);
            let (status, headers , body) = cached.get_parts();
            let response = get_response(status, headers.clone(), body)
            .await.map_err(|_| "failed")?;
            Ok(response)
        } else {
            let client = reqwest::Client::new();
            let (status, headers, bytes) = client.request(method.clone(), url.clone().to_string()).headers(req_headers.clone()).send().await
                    .map(|r| (r.status(), r.headers().clone(), r.bytes()))
                    .map_err(|_|"failed")
                    .unwrap();
            let body = bytes.await.into_diagnostic().map_err(|_|"failed to parse body");
            insert_into_cache(method.clone(), url.clone(), status, headers.clone(), body.clone().unwrap());
            let response = get_response(status, headers, body.unwrap())
                .await.map_err(|_|"error")?;
            Ok(response)
        }

}


async fn get_response(status: StatusCode, headers : HeaderMap,  bytes : Bytes) -> Result<Response<Body>, String> {
    let body = Body::from(bytes);
    let mut response = Response::new(body);
    *response.status_mut() = status;
    let response_headers = response.headers_mut();
    response_headers.extend(headers.iter().map(|(key, val)| (key.clone(), val.clone())));
    Ok(response)
}