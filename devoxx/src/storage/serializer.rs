use std::io::Bytes;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

//use axum::body::Bytes;
use axum::http::{Method, Uri};
use axum::http::StatusCode;

use crate::cache::cache_util::{CacheKey, CachedResponse};

use super::store::{parse_headers, Key, Value};

pub trait Serializer<T> {
    fn serialize(t: T) -> Self;
    fn deserialize(&self) -> T; 
}


pub fn cachekey_to_key(cacheKey : CacheKey) -> Key { 
    Key {
        method : cacheKey.0.to_string(),
        url: cacheKey.1.to_string()
    }
}


pub fn key_to_cachekey(key : Key) -> CacheKey { 
    let method : Method = key.method.parse().unwrap();
    let uri : Uri = key.url.parse().unwrap();
    CacheKey(method, uri)
}


pub fn cached_response_to_value(response : CachedResponse) -> Value { 
    let mut header_str = String::new();
    println!("headerMap : {:?}", response.headers.clone());
    let header_len = response.headers.len();
    for (i, (name, val) ) in response.headers.iter().enumerate() { 
        let h = format!("{}:{}", name, val.to_str().unwrap());
        if i == header_len - 1 { 
            header_str.push_str(h.as_str());
        } else { 
            header_str.push_str(h.as_str());
            header_str.push('\n');
        }
    }
    let plain_bytes: Vec<u8> = response.body.into_iter().collect();
    let body = String::from_utf8(plain_bytes).unwrap();
    Value { 
        status: response.status.as_u16() as i32,
        headers : header_str, 
        body: body, 
        cached_at: response.cached_at.duration_since(UNIX_EPOCH).unwrap().as_secs().to_string()
    }
}


pub fn value_to_cache_response(value : Value) -> CachedResponse{
    let headers = parse_headers(value.headers);
    let status = StatusCode::from_u16(value.status as u16).unwrap();
    let body_bytes  = value.body.as_bytes().to_vec();
    let body = axum::body::Bytes::from(body_bytes);
    CachedResponse { 
        status, 
        headers, 
        body,
        cached_at: SystemTime::UNIX_EPOCH + Duration::from_secs(value.cached_at.parse::<u64>().unwrap())
    }

}