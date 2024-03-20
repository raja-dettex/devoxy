

#[cfg(test)]
mod storeTests { 
    
    use crate::{storage::{serializer::Serializer, store::*}, CacheKey, CachedResponse};
    use std::{fs::OpenOptions, str::FromStr, time::SystemTime};
    use axum::{body::{self, Body, Bytes, HttpBody}, extract::Host, http::{method, uri::{self, PathAndQuery}, HeaderMap, HeaderName, HeaderValue, Method, Request, Response, StatusCode, Uri}, response::IntoResponse, routing::*, RequestExt, Router};


    #[tokio::test]
    async fn test_db_store() { 

        let db_path = "D:/rust-project/devoxy/devoxx/cache.db";
        let db_url = { 
            let file = OpenOptions::new().read(true).write(true).create(true).open(db_path);
            match file { 
                Ok(File) => {
                    println!("file :{:#?}", File);
                    format!("sqlite:{}", db_path)
                }
                Err(err) => {
                    println!("error : {}", err.to_string());
                    "sqlite::memory:".to_string()
                }
            }
            
        };

        println!("url : {}", db_url);


        let mut store = DbStore::new(db_url).await.map_err(|err| err.to_string()).unwrap();
        println!("store is {:#?}", store);
       // store.test_conn().await;
        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("hello").unwrap(), HeaderValue::from_str("rosy").unwrap());
        headers.insert(HeaderName::from_str("jack").unwrap(), HeaderValue::from_str("world").unwrap());
        let cache_key = CacheKey::new(Method::from_str("GET").unwrap(), Uri::from_str("http://localhost:8080").unwrap());
        let cached_response = CachedResponse::new(StatusCode::from_u16(201).unwrap(), headers, Bytes::from_iter(vec![0, 1, 2]), SystemTime::now());
        let res = store.add(cache_key.clone(), cached_response).await;
        if let Err(e) = res { 
            println!("error adding to table : {}", e);
        }
        let result = store.find_page_and_content(cache_key).await;
        println!("content  :{:#?}", result);
       
        
    }

    #[test]
    fn test_parseHeader() { 
        let mut map = HeaderMap::new();
        map.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("text/html; charset=utf-8").unwrap());
        map.insert(HeaderName::from_str("cache-control").unwrap(), HeaderValue::from_str("max-age=3700").unwrap());
        map.insert(HeaderName::from_str("content-length").unwrap(), HeaderValue::from_str("274").unwrap());
        map.insert(HeaderName::from_str("date").unwrap(), HeaderValue::from_str("Tue, 19 Mar 2024 05:24:49 GMT").unwrap());
        println!("headermap : {:?}", map);
        let status = StatusCode::from_u16(201);
        let body = Bytes::from_iter(vec![0, 1, 2]);
        let cached = CachedResponse::new(status.unwrap(), map.clone(), body.clone(), SystemTime::now());
        let content = Page_content::serialize(cached);
        println!("content : {:?}", content);
        let headers = parse_headers(content.headers);
        println!("Retrieved : {:?}", headers);
    }

}