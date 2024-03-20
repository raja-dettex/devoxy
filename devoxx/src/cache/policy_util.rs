use axum::http::HeaderMap;
use core::panic;
use std::time::{Duration, SystemTime};
pub struct CachePolicy { 
    pub headers : HeaderMap   
}

impl CachePolicy { 
    pub fn new(headers : HeaderMap ) -> Self { 
        CachePolicy{headers}
    }
    pub fn is_cacheable(&self) -> bool { 
        let value = self.max_age();
        if let Some(age ) = value { 
            if age > 0 { 
                return true;
            } else { 
                return false;
            }
        }
        false
    }

    pub fn is_stale(&self, time_when_cached: SystemTime) -> bool {
        if let Some(max_age) = self.max_age() {
            // Calculate the current time
            let current_time = SystemTime::now();
            // Calculate the expiration time by adding the maximum age to the time when the resource was cached
            let expiration_time = time_when_cached + Duration::from_secs(max_age as u64);
            
            // Check if the current time is after the expiration time
            if let Ok(duration_since) = current_time.duration_since(expiration_time) {
                // If the duration is greater than zero, the resource is stale
                return duration_since > Duration::from_secs(0);
            } else {
                // If duration_since returns an error, the current time is earlier than the expiration time,
                // so the resource is not stale
                return false;
            }
        }
        
        false // Default to not stale if there's no max-age header or error in calculation
    }
    pub fn is_storable_to_disk(&self) -> bool { 
        if let Some(age) = self.max_age() { 
            if age >= 3600 { 
                return true;
            }
            return false;
        }
        false
    }
    fn max_age(&self) -> Option<i32> {
        let cache_control_header = self.headers.get("CACHE-CONTROL");
        if cache_control_header.is_none() { 
            return None;
        }
        if let Some(cache_control_value) = cache_control_header { 
            let cache_control_str = cache_control_value.to_str();
            match cache_control_str { 
               Ok(val) =>  { 
                    println!("value_str : {}" , val);
                    if let Some(value) = val.trim().strip_prefix("max-age=") { 
                        if let Ok(age) = value.parse::<i32>() { 
                            return Some(age);
                        } else { 
                            return None;
                        }
                    } else { 
                        return None;
                    }
                }, 
                Err(err) =>  {
                    return None;
                }
            }
        }
        None
    }
}