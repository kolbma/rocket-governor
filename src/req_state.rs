//! This module provides the [ReqState] which you may get in contact
//! during implementation of [RocketGovernable](super::RocketGovernable).
//!
//! __Available__ only with __feature limit_info__!
//!
//! There might be no need to create an object by yourself.
//!
//! [ReqState::default()] provides an object which might be only useful in the
//! inner library usage.

use super::{NonZeroU32, Quota};
use rocket::Request;

/// `ReqState` is the data struct to handle information about [`Quota`] and
/// limits in the [`Request`] state.
///
/// This info is provided to the client by [HTTP headers](crate::header).
///
/// When implementing
/// [RocketGovernable::limit_info_allow()](super::RocketGovernable::limit_info_allow())
/// there is passed an object `ReqState` as parameter to provide the
/// possibility for individual decisions about setting
/// [HTTP headers](crate::header).
#[derive(Debug)]
pub struct ReqState {
    /// The [`Request::local_cache()`] is initialized on first call of [`ReqState::get_or_default()`].  
    /// This is detected by `is_default`.
    is_default: bool,

    /// The [Quota](super::Quota) of the current [Request].
    pub(crate) quota: Quota,

    /// Number of [Requests](Request) which can be done, before the end point is
    /// limited.
    ///
    /// Limitation starts below `0`.
    pub(crate) request_capacity: u32,
}

impl ReqState {
    /// Create new [`ReqState`] with provided values
    pub(crate) fn new(quota: Quota, request_capacity: u32) -> Self {
        Self {
            is_default: false,
            quota,
            request_capacity,
        }
    }

    /// Retrieve cached [`ReqState`] from [`Request::local_cache()`]
    ///
    /// Be careful, you'll set cache value to [`ReqState::default()`] if there
    /// has been cached no other value before.
    pub(crate) fn get_or_default<'r>(request: &'r Request) -> Option<&'r Self> {
        let state: &ReqState = request.local_cache(|| {
            ReqState::default() // dummy, because we ignore it if not set
        });

        if !state.is_default {
            Some(state)
        } else {
            None
        }
    }

    /// The [Quota](super::Quota) of the current [Request].
    pub fn quota(&self) -> &Quota {
        &self.quota
    }

    /// Number of [Requests](Request) which can be done, before the end point is
    /// limited.
    ///
    /// Limitation starts below `0`.
    pub fn request_capacity(&self) -> u32 {
        self.request_capacity
    }
}

impl Default for ReqState {
    /// No meaningful implementation.  
    /// Just to provide some default to get `ReqState::get_or_default()` to
    /// work.
    fn default() -> Self {
        Self {
            is_default: true,
            quota: Quota::per_second(NonZeroU32::new(1).unwrap()),
            request_capacity: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::{
        get,
        http::{Header, Status},
        local::blocking::Client,
        routes, Build, Rocket,
    };

    #[get("/")]
    fn route_test() -> Status {
        Status::Ok
    }

    fn launch_rocket() -> Rocket<Build> {
        rocket::build().mount("/", routes![route_test])
    }

    #[test]
    fn test_req_state() {
        let client = Client::untracked(launch_rocket()).expect("no rocket instance");
        let mut req = client.get("/");
        req.add_header(Header::new("X-Real-IP", "127.1.1.1"));
        // req.dispatch();
        let request = req.inner_mut();
        let _ = request.local_cache(|| ReqState::new(Quota::per_second(NonZeroU32::new(1).unwrap()), 10));
        let _ = request.real_ip();
        let req_state = ReqState::get_or_default(request);

        assert!(req_state.is_some());
        assert_eq!(req_state.unwrap().request_capacity, 10);

        // 2nd time
        let req_state = ReqState::get_or_default(request);

        assert!(req_state.is_some());
        assert_eq!(req_state.unwrap().request_capacity, 10);

        // default is none
        let mut req = client.get("/");
        req.add_header(Header::new("X-Real-IP", "127.1.1.2"));
        let request = req.inner_mut();
        let req_state = ReqState::get_or_default(request);

        assert!(req_state.is_none());
    }
}
