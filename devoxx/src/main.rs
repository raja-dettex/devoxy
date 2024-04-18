mod cache;
mod storage;

use core::panic;
use std::{borrow::Borrow, clone, collections::HashMap, env::vars, error::Error, fs::OpenOptions, hash::Hash, io::Read, net::SocketAddr, sync::{Arc, Mutex}, thread, time};
use std::time::{SystemTime, Duration};
use axum::{body::{self, Body, HttpBody,  Bytes}, extract::Host, http::{method, uri::{self, PathAndQuery}, HeaderMap, Request, Response, StatusCode, Uri}, response::IntoResponse, routing::*, RequestExt, Router};
use miette::IntoDiagnostic;
use axum::extract::State;
use reqwest::Method;
use lazy_static::lazy_static;
use cache::{buffer::Buffer, cache::{cacheableBody, RemoteCacheStore}, policy_util::CachePolicy};
use storage::{serializer::{cached_response_to_value, cachekey_to_key}, store::DbStore};
use cache::cache_util::{get_from_cache, insert_into_cache, CacheKey, CachedResponse, is_cached};


#[derive(Debug, Clone)]
struct AppState {
    pub store : DbStore,
    pub cacheStore : RemoteCacheStore,
    pub memMap : Buffer
}


const PROXY_ORGIN_URI : &'static str = "localhost:3000";
const PROXY_FROM_DOMAIN : &'static str = "client.hello";
const DEFAULT_PATH : &'static str = "D:/rust-project/devoxy/devoxx/cache.db";
//let memory_map = Buffer::new();
#[tokio::main]
async fn main() -> Result<(), &'static str> {
    let vars: Vec<_> = std::env::args().collect();
    let file_path = { 
        if let Some(path) = vars.get(1) { 
            path
        } else { 
            DEFAULT_PATH
        }
    };
    println!("file path is {}", file_path);
    let db_url = { 
        let file = OpenOptions::new().read(true).write(true).create(true).open(file_path);
        match file { 
            Ok(File) => {
                println!("file :{:#?}", File);
                format!("sqlite:{}", file_path)
            }
            Err(err) => {
                println!("error : {}", err.to_string());
                "sqlite::memory:".to_string()
            }
        }
        
    };


    let remote_cache_store = RemoteCacheStore::new("redis://localhost:6379".to_string(), 5);
    let memMap = Buffer::new();
    let mut app_state = AppState { store : DbStore::new(db_url).await.unwrap(), cacheStore: remote_cache_store, memMap};
    let cloned_state = app_state.clone();
    let app = Router::new().fallback(|request: Request<Body>| async {
        let response = proxy_handler(request, cloned_state)
        .await
        .map_err(|_| "error").unwrap();
        response
    });


    
    
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    println!("server listening on {}",addr);

    axum::Server::bind(&addr).serve(app.into_make_service()).await.into_diagnostic().map_err(|_| "error".to_string());
    

    
    Ok(())
}

async fn proxy_handler( mut request: Request<Body>, mut state:  AppState) -> Result<Response<Body>, String> {
    let uri : Uri = request.extract_parts().await.unwrap(); 
    let method : Method = request.extract_parts().await.unwrap(); 
    let host: Host = request.extract_parts().await.unwrap();
    let req_headers: HeaderMap = request.extract_parts().await.unwrap();
    
    let split : Vec<_>= host.0.split(':').collect(); 
    let host_name = split[0];
    //println!("host :{}", host_name);
    // if host_name != PROXY_FROM_DOMAIN {
    //     return Err(format!("expected host {} but found {:#?}", PROXY_FROM_DOMAIN.to_string(), host));
    // }
    //let path = uri.path_and_query().cloned().map(|pq| pq.path()).unwrap_or("/");
    let p_and_q = uri.path_and_query().cloned().unwrap_or_else(|| PathAndQuery::from_static("/"));
    let url  = uri::Builder::new().scheme("http")
        .authority(PROXY_ORGIN_URI)
        .path_and_query(p_and_q.clone())
        .build()
        .map_err(|_| "could not build url")?;
    let axum_response = get_cached_response(method, url, req_headers, state).await.map_err(|_| "failed to get cached response")?;
    Ok(axum_response)
}



async fn get_cached_response( method : Method, url: Uri, req_headers : HeaderMap, mut state : AppState) -> Result<Response<Body>, String> {
    // todo 1.check the cache, if the response is in the cache return here
        if state.memMap.is_cached(method.clone(), url.clone()) {
            println!("found");
            let mut cached_content = state.store.find_page_and_content(CacheKey::new(method.clone(), url.clone())).await;
            let response = get_response(cached_content.status, cached_content.headers, cached_content.body).await.unwrap();
            Ok(response)
            // let mut cached_response = get_from_cache(method.clone(), url.clone());
            // let mut cached = Arc::make_mut(&mut cached_response);
            // let ( status, headers, body, cached_at) = cached.get_parts();
            // let response = get_response(status, headers.clone(), body)
            // .await.map_err(|_| "failed")?;
            // Ok(response)
        } else {
            let cache_key = CacheKey::new(method.clone(), url.clone());
            let key = cachekey_to_key(cache_key.clone());
            let res = state.cacheStore.get(key.clone());
            if let Ok(cachedResponse) = res { 
                let status = cachedResponse.status;
                let headers = cachedResponse.headers;
                let body = cachedResponse.body;
                state.memMap.insert_into_cache(method.clone(), url.clone(), status.clone(), headers.clone(), body.clone()).await;
                let response = get_response(status, headers, body).await.map_err(|err| err.to_string()).unwrap();
                return Ok(response);
            }
            let client = reqwest::Client::new();
            let (status, headers, bytes) = client.request(method.clone(), url.clone().to_string()).headers(req_headers.clone()).send().await
                    .map(|r| (r.status(), r.headers().clone(), r.bytes()))
                    .map_err(|_|"failed")
                    .unwrap();
            
            let body = bytes.await.into_diagnostic().map_err(|_|"failed to parse body");
            let cachedRespone = CachedResponse::new(status, headers.clone(), body.clone().unwrap(), SystemTime::now());
            let value  = cached_response_to_value(cachedRespone);
            println!("key is {:#?} value is {:#?}", key, value);
            let cacheable = cacheableBody {key, value};
            println!("cachedable body is {:#?}", cacheable);
            let result = state.cacheStore.set(cacheable);
            match result {
                Ok(str) => println!("added to cache" ),
                Err(err) => println!("error result : {}", err),
            }
            let policy = CachePolicy::new(headers.clone());
            if policy.is_cacheable() { 
                insert_into_cache(method.clone(), url.clone(), status, headers.clone(), body.clone().unwrap());
                println!("cacheable")
            } else { 
                println!("not cacheable")
            }
            if policy.is_storable_to_disk() { 
                let key = CacheKey::new(method.clone(), url.clone());
                let content = CachedResponse::new(status, headers.clone(), body.clone().unwrap(), SystemTime::now());
                let added = state.store.add(key, content).await;
                match added {
                    Ok(_) => println!("added"),
                    Err(err) => println!("error adding to disk : {}", err),
                }
                
            }
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