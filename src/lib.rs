//! # rocket-governor - rate-limiting implementation for Rocket web framework
//!
//! Provides [rocket] guards implementing rate-limiting (based on [governor]).
//!
//! It is used with the derive macro [rocket_governor_derive].
//!
//! ## Example
//!
//! ```rust
//! use rocket::{catchers, get, http::Status, launch, routes};
//! use rocket_governor::{Method, Quota, RocketGovernable};
//! use rocket_governor_derive::RocketGovernor;
//!
//! #[derive(RocketGovernor)]
//! pub struct RateLimitGuard;
//!
//! impl<'r> RocketGovernable<'r> for RateLimitGuard {
//!     fn quota(_method: Method, _route_name: &str) -> Quota {
//!         Quota::per_second(Self::nonzero(1u32))
//!     }
//! }
//!
//! #[get("/")]
//! fn route_example(_limitguard: RateLimitGuard) -> Status {
//!     Status::Ok
//! }
//!
//! #[launch]
//! fn launch_rocket() -> _ {
//!     rocket::build()
//!         .mount("/", routes![route_example])
//!         .register("/", catchers![ratelimitguard_rocket_governor_catcher])
//! }
//! ```
//!
//! See [rocket_governor] Github project for more information.
//!
//! ## Features
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
//! [rocket_governor]: https://github.com/kolbma/rocket_governor/
//! [rocket_governor_derive]: https://docs.rs/rocket_governor_derive/

#![deny(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::all)]

use governor::clock::{Clock, DefaultClock};
pub use governor::Quota;
use header::Header;
use lazy_static::lazy_static;
use logger::{error, info, trace};
use registry::Registry;
pub use rocket::http::Method;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    response::{self, Responder},
    Request, Response,
};
use std::marker::PhantomData;
pub use std::num::NonZeroU32;

pub mod header;
mod logger;
mod registry;

/// The RocketGovernable guard trait.
///
/// [rocket_governor] is a [rocket] guard implementation of the
/// [governor] rate limiter.
///
/// It is used with the derive macro [rocket_governor_derive].
///
/// Declare a struct with `#[derive(RocketGovernor)]` and implement the
/// missing methods of this trait.
///
/// [governor]: https://docs.rs/governor/
/// [rocket]: https://docs.rs/rocket/
/// [rocket_governor]: https://docs.rs/rocket_governor/
/// [rocket_governor_derive]: https://docs.rs/rocket_governor_derive/
///
#[rocket::async_trait]
pub trait RocketGovernable<'r>: FromRequest<'r> + Default {
    /// Returns the [Quota] of the [rocket_governor]
    ///
    /// This is called only once per method/route_name combination.
    /// So it makes only sense to return always the same [Quota] for
    /// equal parameter combinations and no dynamic calculation.
    ///
    /// This is also the requirement to have correct information set
    /// in HTTP headers by registered `rocket_governor_catcher(&Request)`.
    ///
    /// [Quota]: https://docs.rs/governor/latest/governor/struct.Quota.html
    /// [rocket_governor]: https://docs.rs/rocket_governor/
    #[must_use]
    fn quota(method: Method, route_name: &str) -> Quota;

    /// Implementation of catcher used in derive macro
    #[doc(hidden)]
    #[inline(always)]
    fn rocket_governor_catcher<'c>(request: &'c Request) -> &'c LimitError {
        let cached_res: &Result<(), LimitError> = request.local_cache(|| Err(LimitError::Error));
        if let Err(limit_err) = cached_res {
            limit_err
        } else {
            &LimitError::Error
        }
    }

    /// Converts a non-zero number [u32] to [NonZeroU32]
    ///
    /// Number zero/0 becomes 1
    ///
    /// [u32]: https://doc.rust-lang.org/std/primitive.u32.html
    /// [NonZeroU32]: https://doc.rust-lang.org/std/num/struct.NonZeroU32.html
    #[inline]
    fn nonzero(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap_or_else(|| NonZeroU32::new(1u32).unwrap())
    }
}

lazy_static! {
    static ref CLOCK: DefaultClock = DefaultClock::default();
}

/// Helper utility for derive macro [rocket_governor_derive::RocketGovernor].
///
/// [rocket_governor_derive::RocketGovernor]: https://docs.rs/rocket-governor-derive/latest/rocket_governor_derive/
#[doc(hidden)]
pub struct RocketGovernorMacroUtil<'r, T: RocketGovernable<'r>> {
    _phantom: &'r PhantomData<T>,
}

impl<'r, T> RocketGovernorMacroUtil<'r, T>
where
    T: RocketGovernable<'r>,
{
    #[inline(always)]
    pub fn handle_from_request(request: &'r Request) -> Outcome<T, LimitError> {
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
            Outcome::Success(T::default())
        }
    }
}

/// Errors in govern (rate limit).
#[derive(Clone, Debug)]
pub enum LimitError {
    /// Any other undefined LimitError
    Error,

    /// Governed request for the next provided seconds.
    GovernedRequest(u64),

    /// There is no remote client IP address known in the request. Might be a misconfigured server environment.
    MissingClientIpAddr,

    /// Route is not available which might be only the case in fairings
    MissingRoute,

    /// There is a route without name and this can not be matched for rate limiting
    MissingRouteName,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for &LimitError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let mut builder = Response::build();
        let mut builder = builder.status(Status::TooManyRequests);
        builder = match self {
            LimitError::Error => builder.header(Header::XRateLimitError("rate limiter error")),
            LimitError::GovernedRequest(wait_time) => builder
                .header(Header::RetryAfter(*wait_time))
                .header(Header::XRateLimitReset(*wait_time)),
            LimitError::MissingClientIpAddr => builder.header(Header::XRateLimitError(
                "application not retrieving client ip",
            )),
            LimitError::MissingRoute => builder.header(Header::XRateLimitError("routing failure")),
            LimitError::MissingRouteName => {
                builder.header(Header::XRateLimitError("route without name"))
            }
        };
        builder.ok()
    }
}
