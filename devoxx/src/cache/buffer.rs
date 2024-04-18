use std::{collections::HashMap, sync::{Arc, Mutex}, time::SystemTime};

use super::cache_util::{CacheKey, CachedResponse};
use axum::{body::{self, Body, HttpBody,  Bytes}, extract::Host, http::{method, Method, uri::{self, PathAndQuery}, HeaderMap, Request, Response, StatusCode, Uri}, response::IntoResponse, routing::*, RequestExt, Router};

#[derive(Debug)]
pub struct Buffer { 
    Cache : Mutex<HashMap<CacheKey, Arc<CachedResponse>>>
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        let cloned_cache = Mutex::new(self.Cache.lock().unwrap().clone());
        Self { Cache: cloned_cache }
    }
}


impl Buffer { 
    pub fn new() -> Self { 
        let mut hashMap = HashMap::new();
        let mut cache = Mutex::new(hashMap);
        Buffer{Cache:cache}

    }

    pub async fn insert_into_cache(&mut self, method: Method, uri: Uri, status: StatusCode, headers : HeaderMap, body : Bytes) {
        let fresh_cache_key = CacheKey::new(method.clone(), uri.clone());
        let cache_obj = CachedResponse::new(status, headers.clone(), body ,SystemTime::now());
        self.Cache.lock().unwrap().insert(fresh_cache_key, Arc::new(cache_obj));
    }
    
    pub async fn get_from_cache(&self, method: Method, uri: Uri) -> Arc<CachedResponse>{
        let fresh_cache_key = CacheKey::new(method.clone(), uri.clone());
        let cache = self.Cache.lock().unwrap();
        let response = cache.get(&fresh_cache_key).unwrap();
        response.clone()
    }
    
    pub fn is_cached(&self, method: Method, uri: Uri) -> bool{
        let fresh_cache_key = CacheKey::new(method.clone(), uri.clone());
        let exists = self.Cache.lock().unwrap().contains_key(&fresh_cache_key);
        exists
    }
}