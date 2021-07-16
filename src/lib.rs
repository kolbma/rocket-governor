//! Useable for [rocket] guards implementing rate-limiting (based on [governor])
//!
//! See [rocket_governor] for more information.
//!
//! [governor]: https://docs.rs/governor/
//! [rocket]: https://docs.rs/rocket/
//! [rocket_governor]: https://docs.rs/rocket_governor/
//! [rocket_governor_derive]: https://docs.rs/rocket_governor_derive/

#![deny(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::all)]

use governor::clock::{Clock, DefaultClock};
pub use governor::Quota;
use lazy_static::lazy_static;
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

mod registry;

/// [rocket_governor] is a [rocket] guard implementation of the
/// [governor] rate limiter.
///
/// It is used with the derive macro [rocket_governor_derive].
///
/// Declare a struct with `#[derive(RocketGovernor)]` and implement the
/// missing methods.
///
/// [gorvernor]: https://docs.rs/governor/
/// [rocket]: https://docs.rs/rocket/
/// [rocket_governor]: https://docs.rs/rocket_governor/
/// [rocket_governor_derive]: https://docs.rs/rocket_governor_derive/
///
#[rocket::async_trait]
pub trait RocketGovernable<'r>: FromRequest<'r> + Default {
    /// Returns the [Quota] of the [rocket_governor]
    ///
    /// This is called only once per method/route_name combination.
    /// So it makes only sense to return always the same [Quota] for this
    /// combination and no dynamic calculation.
    ///
    /// This is also the requirement to have correct information set
    /// in HTTP headers by registered `too_many_requests_catcher(&Request)`.
    ///
    /// [Quota]: https://docs.rs/governor/latest/governor/struct.Quota.html
    /// [rocket_governor]: https://docs.rs/rocket_governor/
    #[must_use]
    fn quota(method: Method, route_name: &str) -> Quota;

    // #[catch(429)]
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

/// custom header for reporting problems with rate limiter
const HEADER_X_RATELIMIT_ERROR: &'static str = "X-RateLimit-Error";
// TODO: not sure how the different Quota stuff can be handled
//       Would be nice if Quota would return its setting
/// header provides information about limitation of the route
// const HEADER_X_RATELIMIT_LIMIT: &'static str = "X-RateLimit-Limit";
/// header provides information about how many requests are left for the endpoint
// const HEADER_X_RATELIMIT_REMAINING: &'static str = "X-RateLimit-Remaining";
/// header provides the time in seconds when a request to the route is not rate limited
const HEADER_X_RATELIMIT_RESET: &'static str = "X-RateLimit-Reset";

/// Helper utility for derive macro [rocket_governor_derive::RocketGovernor]
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
                    let limiter = Registry::get_or_insert(
                        route.method,
                        route_name,
                        T::quota(route.method, route_name),
                    );
                    if let Some(client_ip) = request.client_ip() {
                        if let Err(notuntil) = limiter.check_key(&client_ip) {
                            let wait_time = notuntil.wait_time_from(CLOCK.now()).as_secs();
                            Err(LimitError::GovernedRequest(wait_time))
                        } else {
                            Ok(())
                        }
                    } else {
                        Err(LimitError::MissingClientIpAddr)
                    }
                } else {
                    Err(LimitError::MissingRouteName)
                }
            } else {
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
            LimitError::Error => builder.raw_header(HEADER_X_RATELIMIT_ERROR, "rate limiter error"),
            LimitError::GovernedRequest(wait_time) => {
                builder.raw_header(HEADER_X_RATELIMIT_RESET, wait_time.to_string())
            }
            LimitError::MissingClientIpAddr => builder.raw_header(
                HEADER_X_RATELIMIT_ERROR,
                "application no retrieving client ip",
            ),
            LimitError::MissingRoute => {
                builder.raw_header(HEADER_X_RATELIMIT_ERROR, "routing failure")
            }
            LimitError::MissingRouteName => {
                builder.raw_header(HEADER_X_RATELIMIT_ERROR, "route without name")
            }
        };
        builder.ok()
    }
}
