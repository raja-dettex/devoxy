

use std::{fmt::format, str::FromStr};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::http::HeaderValue;
use axum::{body::{self, Body, Bytes, HttpBody}, extract::Host, http::{method, uri::{self, PathAndQuery}, HeaderMap, HeaderName, Method, Request, Response, StatusCode, Uri}, response::IntoResponse, routing::*, RequestExt, Router};

use miette::IntoDiagnostic;
use redis::{FromRedisValue, RedisResult, ToRedisArgs};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnection, SqlitePool, SqliteRow };
use sqlx::{query, Execute, Executor };
use uuid::{uuid, Uuid};
use sqlx::{Row, Column};
use hex::encode;
use sqlx::types::Uuid as UUID;

use crate::cache::cache_util::{CacheKey, CachedResponse, Cache};

use super::serializer::Serializer;


#[derive(Debug, Clone)]
pub struct DbStore { 
    //pub options:  ConnectionOptions,
    pub pool : SqlitePool
}

#[derive(Debug)]
pub struct Page { 
    pub id : Option<i32>,
    pub method: String,
    pub url : String
}


#[derive(Debug)]
pub struct Page_content { 
    pub id: Option<i32>,
    pub status: i32,
    pub headers : String,
    pub body : Vec<u8>,
    pub cached_at : String,
    pub page_key : Option<i32>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Key { 
    pub method: String,
    pub url : String
}


impl FromRedisValue for Key {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        match v {
            redis::Value::Data(data) => {
                let result : Key = serde_json::from_slice(data).unwrap();
                Ok(result)
            },
            _ => Err(redis::RedisError::from((redis::ErrorKind::TypeError, "type error for key")))
            
        }
    }
}
impl ToRedisArgs for Key {
    fn write_redis_args<W>(&self, out: &mut W) 
    where
        W: ?Sized + redis::RedisWrite  {
        let json_bytes = serde_json::to_vec(self).unwrap();
        out.write_arg(&json_bytes);
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Value { 
    pub status: i32,
    pub headers : String,
    pub body : String,
    pub cached_at : String,
}

impl Serializer<CacheKey> for Page {
    fn serialize(t: CacheKey) -> Self {
        Page { 
            id: None,
            method: t.0.to_string(),
            url : t.1.to_string()
        }
    }

    fn deserialize(&self) -> CacheKey {
        let method: Method = self.method.parse().unwrap(); // Assuming self.method is a valid HTTP method string
        let uri: Uri = self.url.parse().unwrap(); // Assuming self.url is a valid URI string
        CacheKey::new(method, uri)
    }
}

impl Serializer<CachedResponse> for Page_content {
    fn serialize(t: CachedResponse) -> Self {
        let mut header_str = String::new();
        println!("headerMap : {:?}", t.headers.clone());
        let header_len = t.headers.len();
        for (i, (name, val) ) in t.headers.iter().enumerate() { 
            let h = format!("{}:{}", name, val.to_str().unwrap());
            if i == header_len - 1 { 
                header_str.push_str(h.as_str());
            } else { 
                header_str.push_str(h.as_str());
                header_str.push('\n');
            }
        }
        let plain_bytes: Vec<u8> = t.body.into_iter().collect();
        println!("when serializiing the content length is {}", plain_bytes.clone().len());
         Page_content{ 
            id: None,
            status: t.status.as_u16() as i32,
            headers: header_str,
            body: plain_bytes,
            cached_at: t.cached_at.duration_since(UNIX_EPOCH).unwrap().as_secs().to_string(),
            page_key: None
        }
    }

    fn deserialize(&self) -> CachedResponse {
       let status  = StatusCode::from_u16(self.status as u16).unwrap();
       let headers = parse_headers(self.headers.clone());
       let cached_at = SystemTime::UNIX_EPOCH + Duration::from_secs(self.cached_at.parse::<u64>().unwrap());
       println!("actual content-length : {}", self.body.clone().len());
       let body = axum::body::Bytes::from_iter(self.body.clone());
       CachedResponse::new(status, headers, body, cached_at)
    }
}

pub fn parse_headers(headers_str: String) -> HeaderMap { 
    let mut header_map = HeaderMap::new();
    for line in headers_str.lines() {
        if let Some(index) = line.find(':') {
            let (name, value) = line.split_at(index);
            let name = name.trim();
            let value = value[1..].trim(); // Skipping the ':'
            if !name.is_empty() && !value.is_empty() {
                if let Ok(name) = HeaderName::from_bytes(name.as_bytes()) {
                    if let Ok(value) = HeaderValue::from_str(value) {
                        header_map.insert(name, value);
                    }
                }
            }
        }
    }
    header_map
}

pub type ConnectionOptions = String;

impl DbStore { 
    pub async fn new(db_url : String) -> Result<DbStore, String> { 
        let pool = SqlitePool::connect(&db_url).await.into_diagnostic().map_err(|err| err.to_string());
        if let Ok(db_pool) = pool { 
            let a = sqlx::migrate!("./migrations/").run(&db_pool).await;
            match a { 
                Ok(_) => println!("migrations successfull"),
                Err(err) => println!("migration error : {}", err.to_string())
            }
            Ok(DbStore{pool: db_pool})
        } else if let Err(err)  = pool { 
            println!("error migrating : {}", err);
            Err(err)
        } else { 
            Err("dsf".to_string())
        }
    }

    pub async fn test_conn(&self)  {
        let result = query("SELECT 1 FROM 1;").fetch_one(&self.pool).await.unwrap();
        
        for col in result.columns() { 
            println!("col : {:#?}",col);
            println!("name : {:#?}", col.name());
            let val : i32 = result.get(col.ordinal());
            println!("value : {}", val);
        }
    }


    pub async  fn add(&mut self, key: CacheKey, content : CachedResponse) -> Result<(), String>{ 
        let page = Page::serialize(key);
        let mut page_content = Page_content::serialize(content);
        //page_content.page_key = Some(page.id);
        let stmt = format!("INSERT INTO Page(method, uri) VALUES('{}', '{}');", page.method, page.url);
        let result = self.pool.execute(stmt.as_str()).await.map_err(|err| err.to_string());
        match result {
            Ok(qResult ) => {
                println!("{:#?}", qResult);
                let id : i32 = query("SELECT * FROM Page where method = ? AND uri = ?").bind(page.method).bind(page.url).fetch_one(&self.pool).await.unwrap().get(0);
                page_content.page_key = Some(id);
                let body_hex = encode(&page_content.body);
                let decoded = hex::decode(body_hex.clone()).unwrap();
                println!("decoded length from body hex is {}", decoded.len());
                let page_id = page_content.page_key.unwrap();
                println!("page id : {}", page_id);
                let stmt = format!("INSERT INTO Page_content( response_status, headers, body, cached_at, page_id) VALUES({}, '{}', ?, '{}', {});", 
                   page_content.status, page_content.headers, page_content.cached_at, page_content.page_key.unwrap());
                let result = sqlx::query(&stmt).bind(page_content.body).execute(&self.pool).await.map_err(|err| err.to_string());
                match result { 
                    Ok(qResult) =>  {
                        println!("{:#?}", qResult);
                        Ok(())
                    }
                    Err(err) => {
                        println!("right here");
                        Err(err)
                    }
                }
            }
            Err(err) => {
                println!("here");
                Err(err)
            }
        }
    }
    pub async fn find_page_and_content(&self , key: CacheKey) -> CachedResponse{ 
        let page = Page::serialize(key);
        let query_str = format!("SELECT * FROM Page WHERE method = '{}' AND uri = '{}';", page.method, page.url);
        let result = query(query_str.as_str()).fetch_one(&self.pool).await.map_err(|err| err.to_string()).unwrap();
        //println!("result :{:?}", result);
        let id: i32= result.get_unchecked(0);
        println!("uuid : {}", id);
        let content_query = format!("SELECT * FROM Page_content WHERE page_id = '{}';", id);
        let content_row = query(&content_query).fetch_one(&self.pool).await.map_err(|err| err.to_string()).unwrap();
        let content_id: i32 = content_row.get(0);
        let status: i32 = content_row.get(1);
        let headers: String = content_row.get(2);
        let body: Vec<u8> = content_row.get(3);
        println!("retrived content-length : {}", body.clone().len());
        let cached_at: String = content_row.get(4);
        let page_key: i32 = content_row.get(5);
        let content = Page_content { id: Some(content_id), status: status, headers: headers, body: body, cached_at: cached_at, page_key: Some(page_key)};
        //println!("content - {:#?}", content);
        let cached_content = content.deserialize();
        cached_content
    }
}