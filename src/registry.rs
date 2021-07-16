use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, Quota, RateLimiter};
use lazy_static::lazy_static;
use rocket::http::Method;
use std::{
    borrow::Cow,
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, RwLock},
};

pub type RegisteredRateLimiter =
    Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>;

#[derive(Debug)]
pub struct Registry {
    limiter: RwLock<HashMap<Method, HashMap<String, RegisteredRateLimiter>>>,
}

impl Registry {
    pub fn get_or_insert(
        method: Method,
        route_name: &Cow<str>,
        quota: Quota,
    ) -> RegisteredRateLimiter {
        // check if exist with readlock
        let limiter = if let Ok(rlock) = REG.limiter.read() {
            if let Some(meth_found) = rlock.get(&method) {
                if let Some(limiter) = meth_found.get(&**route_name) {
                    Some(Arc::clone(limiter))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // get the existing limiter or create the new one with writelock and return the created
        let limiter = if let Some(limiter) = limiter {
            limiter
        } else {
            let mut wlock = REG.limiter.write().unwrap();
            if let Some(meth_found) = wlock.get_mut(&method) {
                if let Some(limiter) = meth_found.get(&**route_name) {
                    Arc::clone(limiter)
                } else {
                    let limiter = Arc::new(RateLimiter::keyed(quota));
                    meth_found.insert(route_name.to_string(), Arc::clone(&limiter));
                    limiter
                }
            } else {
                let mut lim_map = HashMap::new();
                let limiter = Arc::new(RateLimiter::keyed(quota));
                lim_map.insert(route_name.to_string(), Arc::clone(&limiter));
                wlock.insert(method, lim_map);
                limiter
            }
        };

        // simple cleanup
        limiter.retain_recent();
        limiter.shrink_to_fit();

        limiter
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            limiter: RwLock::new(HashMap::new()),
        }
    }
}

lazy_static! {
    static ref REG: Registry = Registry::default();
}
