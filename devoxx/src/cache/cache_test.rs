

#[cfg(test)]
mod cache_test {
    use std::time::SystemTime;

    use crate::{cache::{cache::{cacheableBody, RemoteCacheStore}, cache_util::CachedResponse}, storage::{serializer::cached_response_to_value, store::{Key, Value}}};
    use axum::http::{HeaderMap, HeaderValue};
    #[test]
    fn test_if_connection_open() { 
        let mut store = RemoteCacheStore::new("redis://localhost:6379".to_string(), 5);
        let key = Key { 
            method : "GET".to_string(), 
            url : "demo".to_string()
        }; 
        let deleted_result = store.remove(key.clone());
        match deleted_result {
            Ok(res) => println!("deleted {}", res ),
            Err(err) => println!("error removing key : {}", err),
        }
        let mut headers = HeaderMap::new();
        headers.insert("key1", HeaderValue::from_str("value1").unwrap());
        let body =axum::body::Bytes::from("hello bytes");
        let cachedResponse = CachedResponse::new(axum::http::StatusCode::from_u16(400).unwrap(), headers, body, SystemTime::now());
        println!("built response : {:#?}", cachedResponse);
        let value = cached_response_to_value(cachedResponse);
        let cacheble = cacheableBody { 
            key : key.clone(),
            value : value
        };
        let res =  store.set(cacheble);
        match res {
            Ok(value) => println!("set successfully"),
            Err(err) => println!("error setting the key : {} ", err),
        }
        let result = store.get(key.clone());
        match result { 
            Ok(res) => println!("res : {:#?}", res),
            Err(err) => println!("error fetching the key : {}", err)
        }
        let cleanup_result = store.remove(key.clone());
        match cleanup_result {
            Ok(res) => println!("deleted {}", res ),
            Err(err) => println!("error removing key : {}", err),
        }
        
    }
    
}