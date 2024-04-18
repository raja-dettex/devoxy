

#[cfg(test)]
mod storeTests { 

    use http::header;
    use serde::{Deserialize, Serialize};
    use serde_json;

    
    
    use crate::{storage::{serializer::{cached_response_to_value, cachekey_to_key, key_to_cachekey, value_to_cache_response, Serializer}, store::*}, CacheKey, CachedResponse};
    use std::{fs::{read, OpenOptions}, io::Read, str::FromStr, time::SystemTime};
    use axum::{body::{self, Body, Bytes, HttpBody}, extract::Host, http::{method, uri::{self, PathAndQuery}, HeaderMap, HeaderName, HeaderValue, Method, Request, Response, StatusCode, Uri}, response::IntoResponse, routing::*, RequestExt, Router};
    use reqwest::Url;
    use serde_json::from_str;

    #[derive(Serialize, Deserialize, Debug)]
    struct cacheableBody { 
        key : Key, 
        value : Value,
    }


    #[derive(Serialize, Deserialize, Clone)]
    struct GetQuery { 
        key : Key
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct User {
        fingerprint: String,
        location: String,
    }

    impl From<GetQuery> for reqwest::Body { 
        fn from(value: GetQuery) -> Self {
            let body_str = serde_json::to_string(&value).unwrap();
            reqwest::Body::from(body_str)
        }
    }

    
    impl From<cacheableBody> for reqwest::Body {
        fn from(value: cacheableBody) -> Self {
            let body_str = serde_json::to_string(&value).unwrap();
            reqwest::Body::from(body_str)
        }
    }

    #[test]
    fn test_serializer() { 
        // let cached_key = CacheKey::new(Method::from_str("GET").unwrap(), Uri::from_str("1270.0.0.1:5432").unwrap());
        // let mut headers = HeaderMap::new();
        // let some_bytes = Bytes::from("hello there");
        // headers.insert(HeaderName::from_str("hello").unwrap(), HeaderValue::from_str("world").unwrap());
        // headers.insert(HeaderName::from_str("jack").unwrap(), HeaderValue::from_str("rose").unwrap());
        // //let body = Body::from(some_bytes);
        // let cached_value = CachedResponse::new(StatusCode::OK, headers , some_bytes.clone(), SystemTime::now());
        // let key = cachekey_to_key(cached_key);
        // let value = cached_response_to_value(cached_value);
        // let cacheable_content = cacheableBody { 
        //     key, value
        // };
        // println!("content is {:#?}" , cacheable_content);
        // let content_str = serde_json::to_string(&cacheable_content).unwrap();
        // println!("content string : {}", content_str);
        // let retrieved = serde_json::from_str::<cacheableBody>(&content_str).unwrap();
        // println!("retrieved value is {:#?}" , retrieved);
        let cached = cacheableBody {
            key: Key {
                method: "GET".to_string(),
                url: "http://localhost:3000/fast".to_string(),
            },
            value: Value {
                status: 200,
                headers: "content-type:text/html; charset=utf-8\ncache-control:max-age=3700\ncontent-length:274\ndate:Mon, 08 Apr 2024 12:43:21 GMT".to_string(),
                body: "<script src=\"https://cdn.tailwindcss.com\"></script><body class=\"flex flex-col items-center justify-center h-screen\"><h1 class=\"text-6xl\">Fast</h1><p class=\"text-4xl\">2024-04-08 12:43:21.034713500 UTC</p><a class=\"text-blue-400 pt-16 text-xl\" href=\"/\">Go back home</a></body>".to_string(),
                cached_at: "1712580201".to_string(),
            },
        };
        let cache_key = key_to_cachekey(cached.key);
        let content = value_to_cache_response(cached.value);
        println!("key is {:#?}, value is {:#?}", cache_key, content);
    }


    #[test]
    fn bytes_to_str() {
        let b = Bytes::from_static(b"helllo"); 
        let iterable : Vec<u8> = b.into_iter().collect();
        println!("iterable is {:#?}", iterable);
        let string = String::from_utf8(iterable).unwrap();
        println!("reverted string is {}", string);
        let bytes = Bytes::from_iter(vec![0,1,2]);
        let str = String::from_utf8_lossy(&bytes).to_string();
        println!("str is {}", str);
    }

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
    fn test_sliceofu8_to_string() { 
        let slice = vec![0, 1, 2, 5, 7];
        let string = String::from_utf8_lossy(&slice).into_owned();
        println!("string is {}", string);
    }

    #[tokio::test]
    async fn test_cache_layer() {
        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("hello").unwrap(), HeaderValue::from_str("rosy").unwrap());
        headers.insert(HeaderName::from_str("jack").unwrap(), HeaderValue::from_str("world").unwrap());
        let cache_key = CacheKey::new(Method::from_str("GET").unwrap(), Uri::from_str("http://localhost:9090/hello").unwrap());
        let cached_response = CachedResponse::new(StatusCode::from_u16(201).unwrap(), headers, Bytes::from_static(b"hello"), SystemTime::now());
        let page = Page::serialize(cache_key);
        let content = Page_content::serialize(cached_response);
        let client = reqwest::Client::new();
        let url = Url::from_str("http://localhost:6000/api/set").unwrap();
        let key = Key { method: page.method, url: page.url};
        //let content_body_str = serde_json::from_slice(&content.body);
        let value = Value {status: content.status, headers : content.headers, body: String::from_utf8(content.body).unwrap() , cached_at: content.cached_at};
        let cacheable = cacheableBody{ key : key.clone(), value : value};
        let b = reqwest::Body::from(cacheable);
        let (status, headerMap, body) = client.request(Method::from_str("POST").unwrap(), url).body(b).send().await
            .map(|r| (r.status().clone(), r.headers().clone(), r.bytes()))
            .map_err(|err| err.to_string())
            .unwrap();
        println!("status code : {}", status.as_u16());
        println!("headers : {:#?}", headerMap);
        let bytes = body.await.unwrap();
        let str = String::from_utf8_lossy(&bytes).into_owned();
        // println!("body is {} string", str);
        // //let bytes = body.await;
        // let j = b"
        // {
        //     \"fingerprint\": \"0xF9BA143B95FF6D82\",
        //     \"location\": \"Menlo Park, CA\"
        // }";
        // let another : User = serde_json::from_slice(j).unwrap();
        // println!("another : {:#?}", another);
        let get_url = Url::from_str("http://localhost:6000/api/get").unwrap();
        let query = GetQuery{key: key.clone()};
        let query_bytes = reqwest::Body::from(query);
        let (get_status, get_headers , get_body) = client.request(Method::from_str("GET").unwrap(), get_url).body(query_bytes).send().await
            .map(|r| (r.status(), r.headers().clone(), r.bytes())).map_err(|err| err.to_string()).unwrap();
        let body_bytes = get_body.await.map_err(|err| err.to_string()).unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes).into_owned();
        println!("get status : {}", get_status.as_u16());
        println!("get headers : {:#?}", get_headers);
        println!("get body str : {}", body_str);
        let cached : cacheableBody = serde_json::from_slice(&body_bytes).map_err(|err| err.to_string()).unwrap();
        
        println!("response body : {:#?}", cached);
        // match bytes { 
        //     Ok(bodyBytes) => {
        //         let result: Result<String, String>   = serde_json::from_slice(&bodyBytes).map_err(|err| err.to_string());
        //         if let Ok(res) = result { 
        //             println!("result is {}", res);
        //         } else if let Err(err) = result { 
        //             println!("error is {}", err)
        //         }
               
        //     }
        //     Err(err) => println!("error : {}", err.to_string())
        // }
        
        //let body: String = jsonBody.await.unwrap();
        
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