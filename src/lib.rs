//! # rocket_governor - rate-limiting implementation for Rocket web framework
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
//! There is the optional feature __logger__ which enables some logging output.
//!
//! For usage depend on it in Cargo.toml
//! ```toml
//! [dependencies]
//! rocket_governor = { version = "...", features = ["logger"] }
//! ```
//!
//! [governor]: https://docs.rs/governor/
//! [rocket]: https://docs.rs/rocket/
//! [rocket-governor]: https://github.com/kolbma/rocket-governor/

#![deny(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::all)]
#![deny(missing_docs)]
#![deny(missing_doc_code_examples)]

use governor::clock::{Clock, DefaultClock};
pub use governor::Quota;
use lazy_static::lazy_static;
pub use limit_error::LimitError;
use logger::{error, info, trace};
use registry::Registry;
pub use rocket::http::Method;
use rocket::{
    async_trait, catch,
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};
use std::marker::PhantomData;
pub use std::num::NonZeroU32;

pub mod header;
mod limit_error;
mod logger;
mod registry;

/// The [RocketGovernable] guard trait.
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
/// [rocket_governor]: https://docs.rs/rocket_governor/
///
#[async_trait]
pub trait RocketGovernable<'r> {
    /// Returns the [Quota] of the [RocketGovernable].
    ///
    /// This is called only once per method/route_name combination.
    /// So it makes only sense to return always the same [Quota] for
    /// equal parameter combinations and no dynamic calculation.
    ///
    /// This is also the requirement to have correct information set
    /// in HTTP headers by registered [`rocket_governor_catcher()`](crate::rocket_governor_catcher()).
    ///
    /// [Quota]: https://docs.rs/governor/latest/governor/struct.Quota.html
    #[must_use]
    fn quota(method: Method, route_name: &str) -> Quota;

    /// Converts a non-zero number [u32] to [NonZeroU32](std::num::NonZeroU32).
    ///
    /// Number zero/0 becomes 1.
    ///
    #[inline]
    fn nonzero(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap_or_else(|| NonZeroU32::new(1u32).unwrap())
    }
}

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
                        if let Err(notuntil) = limiter.check_key(&client_ip) {
                            let wait_time = notuntil.wait_time_from(CLOCK.now()).as_secs();
                            info!(
                                "ip {} method {} route {} limited {} sec",
                                &client_ip, &route.method, route_name, &wait_time
                            );
                            Err(LimitError::GovernedRequest(wait_time))
                        } else {
                            trace!(
                                "not governed ip {} method {} route {}",
                                &client_ip,
                                &route.method,
                                route_name
                            );
                            Ok(())
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

        if let Err(e) = res {
            let e = e.clone();
            match e {
                LimitError::GovernedRequest(_) => Outcome::Failure((Status::TooManyRequests, e)),
                _ => Outcome::Failure((Status::BadRequest, e)),
            }
        } else {
            Outcome::Success(Self::default())
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
