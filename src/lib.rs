//! # rocket-governor - rate-limiting implementation for Rocket web framework
//!
//! Provides the [rocket] guard implementing rate-limiting (based on [governor]).
//!
//! Declare a struct and use it with the generic [RocketGovernor] guard.  
//! This requires to implement trait [RocketGovernable] for your struct.
//!
//! ## Example
//!
//! ```rust
//! use rocket::{catchers, get, http::Status, launch, routes};
//! use rocket_governor::{rocket_governor_catcher, Method, Quota, RocketGovernable, RocketGovernor};
//!
//! pub struct RateLimitGuard;
//!
//! impl<'r> RocketGovernable<'r> for RateLimitGuard {
//!     fn quota(_method: Method, _route_name: &str) -> Quota {
//!         Quota::per_second(Self::nonzero(1u32))
//!     }
//! }
//!
//! #[get("/")]
//! fn route_example(_limitguard: RocketGovernor<RateLimitGuard>) -> Status {
//!     Status::Ok
//! }
//!
//! #[launch]
//! fn launch_rocket() -> _ {
//!     rocket::build()
//!         .mount("/", routes![route_example])
//!         .register("/", catchers![rocket_governor_catcher])
//! }
//! ```
//!
//! See [rocket-governor] Github project for more information.
//!
//! ## Features
//!
//! ### Optional feature __limit_info__
//!
//! There is the optional feature __limit_info__ which enables reporting about
//! rate limits in HTTP headers of requests.
//!
//! The implementation is based on headers of
//! [https://datatracker.ietf.org/doc/html/draft-ietf-httpapi-ratelimit-headers]().
//! The feature provides a default implementation of a Rocket fairing
//! which need to be used to get the HTTP headers set.
//!
//! See API documentation for [LimitHeaderGen](crate::LimitHeaderGen).
//!
//! For usage depend on it in Cargo.toml
//! ```toml
//! [dependencies]
//! rocket-governor = { version = "...", features = ["limit_info"] }
//! ```
//!
//! ### Optional feature __logger__
//!
//! There is the optional feature __logger__ which enables some logging output.
//!
//! For usage depend on it in Cargo.toml
//! ```toml
//! [dependencies]
//! rocket-governor = { version = "...", features = ["logger"] }
//! ```
//!
//! [governor]: https://docs.rs/governor/
//! [rocket]: https://docs.rs/rocket/
//! [rocket-governor]: https://github.com/kolbma/rocket-governor/

#![deny(clippy::all)]
#![deny(keyword_idents)]
#![deny(missing_docs)]
#![deny(non_ascii_idents)]
#![deny(unreachable_pub)]
#![deny(unsafe_code)]
#![deny(unused_crate_dependencies)]
#![deny(unused_qualifications)]
//#![deny(unused_results)]
#![deny(warnings)]

use governor::clock::{Clock, DefaultClock};
pub use governor::Quota;
use lazy_static::lazy_static;
pub use limit_error::LimitError;
#[cfg(feature = "limit_info")]
pub use limit_header_gen::LimitHeaderGen;
use logger::{error, info, trace};
use registry::Registry;
#[cfg(feature = "limit_info")]
pub use req_state::ReqState;
pub use rocket::http::Method;
use rocket::{
    async_trait, catch,
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};
pub use rocket_governable::RocketGovernable;
use std::marker::PhantomData;
pub use std::num::NonZeroU32;

pub mod header;
mod limit_error;
#[cfg(feature = "limit_info")]
mod limit_header_gen;
mod logger;
mod registry;
#[cfg(feature = "limit_info")]
mod req_state;
mod rocket_governable;

/// Generic [RocketGovernor] implementation.
///
/// [rocket_governor](crate) is a [rocket] guard implementation of the
/// [governor] rate limiter.
///
/// Declare a struct and use it with the generic [RocketGovernor] guard.
/// This requires to implement [RocketGovernable] for your struct.
///
/// See the top level [crate] documentation.
///
/// [governor]: https://docs.rs/governor/
/// [rocket]: https://docs.rs/rocket/
///
pub struct RocketGovernor<'r, T>
where
    T: RocketGovernable<'r>,
{
    _phantom: PhantomData<&'r T>,
}

lazy_static! {
    static ref CLOCK: DefaultClock = DefaultClock::default();
}

#[doc(hidden)]
impl<'r, T> RocketGovernor<'r, T>
where
    T: RocketGovernable<'r>,
{
    /// Handler used in `FromRequest::from_request(request: &'r Request)`.
    #[inline(always)]
    pub fn handle_from_request(request: &'r Request) -> Outcome<Self, LimitError> {
        let res = request.local_cache(|| {
            if let Some(route) = request.route() {
                if let Some(route_name) = &route.name {
                    let limiter = Registry::get_or_insert::<T>(
                        route.method,
                        route_name,
                        T::quota(route.method, route_name),
                    );
                    if let Some(client_ip) = request.client_ip() {
                        let limit_check_res = limiter.check_key(&client_ip);
                        match limit_check_res {
                            Ok(state) => {
                                #[allow(unused_variables)] // only used in trace or when feature limit_info
                                let request_capacity = state.remaining_burst_capacity();
                                trace!(
                                    "not governed ip {} method {} route {}: remaining request capacity {}",
                                    &client_ip,
                                    &route.method,
                                    route_name,
                                    request_capacity
                                );

                                #[cfg(feature = "limit_info")] {
                                    // `local_cache` lookup works by type and so it doesn't work to catch
                                    // `LimitError` and handle different Ok objects:
                                    // See https://rocket.rs/v0.5-rc/guide/state/#request-local-state
                                    // State wrapper is so cached separate...
                                    let req_state = ReqState::new(state.quota(), request_capacity);
                                    let is_req_state_allowed = T::limit_info_allow(Some(route.method), Some(route_name), &req_state);
                                    if is_req_state_allowed {
                                        // For safety and speed this is used by default in a limited way, see:
                                        // * Information disclosure:
                                        //   https://datatracker.ietf.org/doc/html/draft-ietf-httpapi-ratelimit-headers#section-6.2
                                        //
                                        let _ = request.local_cache(|| req_state);
                                    }
                                }

                                Ok(()) // needs to be something not changing during request
                            }
                            Err(notuntil) => {
                                let wait_time = notuntil.wait_time_from(CLOCK.now()).as_secs();
                                info!(
                                    "ip {} method {} route {} limited {} sec",
                                    &client_ip, &route.method, route_name, &wait_time
                                );
                                Err(LimitError::GovernedRequest(wait_time, notuntil.quota()))
                            }
                        }
                    } else {
                        error!(
                            "missing ip - method {} route {}: request: {:?}",
                            &route.method, route_name, request
                        );
                        Err(LimitError::MissingClientIpAddr)
                    }
                } else {
                    error!("route without name: request: {:?}", request);
                    Err(LimitError::MissingRouteName)
                }
            } else {
                error!("routing failure: request: {:?}", request);
                Err(LimitError::MissingRoute)
            }
        });

        match res {
            Ok(_) => {
                #[cfg(feature = "limit_info")]
                {
                    // available if `T::limit_info_allow()` is true
                    let state_opt = ReqState::get_or_default(&request);
                    #[allow(unused_variables)] // state only used in trace
                    if let Some(state) = state_opt {
                        trace!(
                            "request_capacity: {} rate-limit: {}",
                            state.request_capacity,
                            state.quota.burst_size().get()
                        );
                    }
                }

                // Forward request
                Outcome::Success(Self::default())
            }
            Err(e) => {
                let e = e.clone();
                match e {
                    LimitError::GovernedRequest(_, _) => {
                        Outcome::Error((Status::TooManyRequests, e))
                    }
                    _ => Outcome::Error((Status::BadRequest, e)),
                }
            }
        }
    }
}

#[doc(hidden)]
impl<'r, T> Default for RocketGovernor<'r, T>
where
    T: RocketGovernable<'r>,
{
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

#[doc(hidden)]
#[async_trait]
impl<'r, T> FromRequest<'r> for RocketGovernor<'r, T>
where
    T: RocketGovernable<'r>,
{
    type Error = LimitError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, LimitError> {
        Self::handle_from_request(request)
    }
}

/// A default implementation for Rocket [Catcher] handling HTTP TooManyRequests responses.
///
/// ## Example
///
/// ```rust
/// use rocket::{catchers, launch};
/// use rocket_governor::rocket_governor_catcher;
///
/// #[launch]
/// fn launch_rocket() -> _ {
///     rocket::build()
///         .register("/", catchers![rocket_governor_catcher])
/// }
/// ```
///
/// [Catcher]: https://api.rocket.rs/v0.5-rc/rocket/struct.Catcher.html
#[catch(429)]
pub fn rocket_governor_catcher<'r>(request: &'r Request) -> &'r LimitError {
    let cached_res: &Result<(), LimitError> = request.local_cache(|| Err(LimitError::Error));
    if let Err(limit_err) = cached_res {
        limit_err
    } else {
        &LimitError::Error
    }
}
