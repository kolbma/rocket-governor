use crate::logger::debug;
use governor::{
    clock::DefaultClock, middleware::StateInformationMiddleware,
    state::keyed::DefaultKeyedStateStore, Quota, RateLimiter,
};
use lazy_static::lazy_static;
use rocket::http::Method;
use std::{
    any::type_name,
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, RwLock},
};

pub(crate) type RegisteredRateLimiter = Arc<
    RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock, StateInformationMiddleware>,
>;

#[derive(Debug)]
pub(crate) struct Registry {
    limiter: RwLock<HashMap<Method, HashMap<String, RegisteredRateLimiter>>>,
}

impl Registry {
    pub(crate) fn get_or_insert<T>(
        method: Method,
        route_name: &str,
        quota: Quota,
    ) -> RegisteredRateLimiter {
        let route_name = type_name::<T>().to_string() + "::" + route_name;

        // check if exist with readlock
        let limiter = if let Ok(rlock) = REG.limiter.read() {
            if let Some(meth_found) = rlock.get(&method) {
                meth_found.get(&route_name).map(|limiter| {
                    debug!("limiter found method {} route {}", method, route_name);
                    Arc::clone(limiter)
                })
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
            let mut wlock_meth_map = REG.limiter.write().unwrap();
            if let Some(meth_found) = wlock_meth_map.get_mut(&method) {
                if let Some(limiter) = meth_found.get(&route_name) {
                    debug!("limiter found method {} route {}", &method, &route_name);
                    Arc::clone(limiter)
                } else {
                    debug!("new limiter method {} route {}", &method, &route_name);
                    let limiter = Arc::new(
                        RateLimiter::keyed(quota).with_middleware::<StateInformationMiddleware>(),
                    );
                    meth_found.insert(route_name, Arc::clone(&limiter));
                    limiter
                }
            } else {
                debug!("new limiter method {} route {}", &method, &route_name);
                let mut lim_map = HashMap::new();
                let limiter = Arc::new(
                    RateLimiter::keyed(quota).with_middleware::<StateInformationMiddleware>(),
                );
                lim_map.insert(route_name, Arc::clone(&limiter));
                wlock_meth_map.insert(method, lim_map);
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
