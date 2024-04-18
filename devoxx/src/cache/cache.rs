use std::{fmt, str::FromStr, sync::{Arc, Mutex}};

use axum::http::request;
use reqwest::Client;
use serde::{de::value, Deserialize, Serialize};

use crate::storage::{serializer::value_to_cache_response, store::{self, Key, Value}};

use super::cache_util::CachedResponse;
use redis::{Client as RedisClient, Commands, FromRedisValue, RedisError, ToRedisArgs};

use redis::Connection;

type RedisConnection = Arc<Mutex<Connection>>;



#[derive(Serialize, Deserialize, Debug)]
    pub struct cacheableBody { 
        pub key : Key, 
        pub value : Value,
        
    }


impl ToRedisArgs for cacheableBody {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite {
        let value = serde_json::to_vec(self).unwrap();
        out.write_arg(&value);
    }
}


impl FromRedisValue for cacheableBody {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        match v { 
            redis::Value::Data(data) => {
                let value : cacheableBody = serde_json::from_slice(data).unwrap();
                Ok(value)
            },
            _ => Err(redis::RedisError::from((redis::ErrorKind::TypeError, "invalid value type")))
            
        }
    }
}


    #[derive(Serialize, Deserialize, Clone)]
    struct GetQuery { 
        key : Key
    }


impl From<GetQuery> for reqwest::Body {
    fn from(value: GetQuery) -> Self {
        let query_str = serde_json::to_string(&value).unwrap();
        let body = reqwest::Body::from(query_str);
        body
    }
}
impl From<cacheableBody> for reqwest::Body {
    fn from(value: cacheableBody) -> Self {
        let str = serde_json::to_string(&value).unwrap();
        let body = reqwest::Body::from(str);
        body
    }
}


#[derive( Clone)]
pub struct RedisPool { 
    pub connections: Vec<RedisConnection>,
}

impl fmt::Debug for RedisPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedisPool")
            .field("connection_count", &self.connections.len())
            .finish()
    }
}

pub fn create_connection(conn_str : String) -> Result<RedisConnection, String> { 
    let client = RedisClient::open(conn_str).map_err(|err| err.to_string()).expect("failed to create client");
    let connection = client.get_connection().map_err(|err| err.to_string());
    match connection  { 
        Ok(conn) => Ok(Arc::new(Mutex::new(conn))),
        Err(err) => { 
            //panic!("error : {:#?}", err);
            println!("err creating connections : {}", err);
            Err(err)
        }
    }
    //Ok(Arc::new(Mutex::new(connection)))
}

impl RedisPool { 
    pub fn new(conn_str : String , max_conn : i32) -> Self { 
        let mut connections = Vec::new();
        for _ in 0..max_conn { 
            if let Ok(conn) = create_connection(conn_str.clone()) { 
                connections.push(conn);
            } 
        }
        RedisPool { connections}
    }
}


#[derive(Debug,Clone)]
pub struct RemoteCacheStore { 
    pool : RedisPool, 
    i : i32,
}

impl RemoteCacheStore { 
    pub fn new(conn_str : String , max_conn : i32) -> Self {
       let pool = RedisPool::new(conn_str, max_conn);
       RemoteCacheStore { pool, i: 0 }
    } 

    pub fn get_conn(&mut self) -> Option<&RedisConnection> { 
        let index = (self.i as usize) % self.pool.connections.len();
        let connection = self.pool.connections.get(index);
        self.i = self.i + 1;
        connection
    }

    pub fn get(&mut self, key : Key) -> Result<CachedResponse, String >{
        let mut conn = self.get_conn();
        if conn.is_none() { 
            conn = self.get_conn();
        }
        let result = conn.unwrap().lock().unwrap().get::<store::Key, cacheableBody>(key).map_err(|err| err.to_string());
        match result { 
            Ok(cacheable) => { 
                let cachedResponse = value_to_cache_response(cacheable.value);
                Ok(cachedResponse)
            },
            Err(err) => Err(err),
        }
    }

    pub fn set(&mut self, cacheable: cacheableBody) -> Result<(), String>{
        let key = cacheable.key.clone();
        let mut conn = self.get_conn().unwrap().lock().unwrap();
        match conn.set(key, cacheable).map_err(|err| err.to_string()) {
            Ok(redis::Value::Okay) =>{
                println!("value was successfully set");
                Ok(())
            },
            Ok(_) => {
                println!("unexpected response");
                Err("unexpected response".to_string())
            }
            Err(err) => Err(err),
        }
        
    }


    pub fn remove(&mut self, key : Key) -> Result<bool, String> { 
        let mut conn = self.get_conn().unwrap().lock().unwrap();
        let result  : Result<bool, String>= conn.del(key).map_err(|err| err.to_string());
        result 
    }
}

