
use std::{collections::HashMap, hash::Hash, sync::{Arc, Mutex}, thread, time::SystemTime, time};

use lazy_static::lazy_static;
use axum::{body::{self, Body, HttpBody,  Bytes}, extract::Host, http::{method, Method, uri::{self, PathAndQuery}, HeaderMap, Request, Response, StatusCode, Uri}, response::IntoResponse, routing::*, RequestExt, Router};
use crate::cache::policy_util::CachePolicy;


#[derive(PartialEq, Eq, Clone, Debug)]
pub struct CacheKey(pub Method,pub Uri);



#[derive(Clone, Debug)]
pub struct CachedResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body : Bytes,
    pub cached_at: SystemTime
}

impl CachedResponse  {
    pub fn default() -> Self { 
        CachedResponse{status: StatusCode::OK, headers: HeaderMap::new(), body: Bytes::new(), cached_at: SystemTime::now()}
    }
    pub fn new(status : StatusCode, headers : HeaderMap, body: Bytes, cached_at : SystemTime) -> Self {
        CachedResponse{ status , headers: headers.clone(), body : body, cached_at}
    }
    pub fn get_parts(&self) -> ( StatusCode, HeaderMap, Bytes,SystemTime) {
        (self.status, self.headers.clone(), self.body.clone(), self.cached_at)
    }
}

pub struct Cache {
    inner: Mutex<HashMap<CacheKey, Arc<CachedResponse>>>
}


impl Hash for CacheKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.hash(state);
    }
}


impl CacheKey {
    pub fn new(method : Method, uri : Uri) -> Self{
        CacheKey(method, uri)
        
    }
}
lazy_static! {
    static ref CACHE : Arc<Cache> = Arc::new(Cache {
        inner: Mutex::new(HashMap::new())
    });
}


pub fn insert_into_cache(method: Method, uri: Uri, status: StatusCode, headers : HeaderMap, body : Bytes) {
    let fresh_cache_key = CacheKey::new(method.clone(), uri.clone());
    let cache_obj = CachedResponse::new(status, headers.clone(), body ,SystemTime::now());
    CACHE.clone().inner.lock().unwrap().insert(fresh_cache_key, Arc::new(cache_obj));
}

pub fn get_from_cache(method: Method, uri: Uri) -> Arc<CachedResponse>{
    let fresh_cache_key = CacheKey::new(method.clone(), uri.clone());
    let cache = CACHE.inner.lock().unwrap();
    let response = cache.get(&fresh_cache_key).unwrap();
    response.clone()
}

pub fn is_cached(method: Method, uri: Uri) -> bool{
    let fresh_cache_key = CacheKey::new(method.clone(), uri.clone());
    let exists = CACHE.inner.lock().unwrap().contains_key(&fresh_cache_key);
    exists
}

pub async fn remove_stale_cache(cache: Arc<Cache>) { 
    println!("here");
    loop { 
        println!("inside the loop");
        let cache_clone = cache.clone();
        for (key ,val) in cache_clone.inner.lock().unwrap().iter() { 
            println!("checking");
            let keepCachePolicy = CachePolicy::new(val.headers.clone());
            if keepCachePolicy.is_stale(val.cached_at) { 
                println!("hereeeee");
                if let Some(i) = remove_from_cache(&cache, key.0.clone(), key.1.clone()) { 
                    println!("removed {}", i);
                } else { 
                    println!("nothing to remove");
                }
                
            }
            println!("exiting");
            
        }
        println!("for exited");
        //tokio::time::sleep(time::Duration::from_secs(30)).await;
        thread::sleep(time::Duration::from_secs(10));
    }
}

pub fn remove_from_cache( cache : &Arc<Cache>, method : Method, uri: Uri) -> Option<u32> {
    println!("here..."); 
    let cacheKey = CacheKey::new(method.clone(), uri.clone());
    let mut cache = CACHE.inner.lock().unwrap();
    if cache.remove(&cacheKey).is_none()  {
        return None;
    } 
    Some(1 as u32)
 }

