//! Errors for governed requests which implement 
//! [Responder](rocket::response::Responder).

use super::{header::Header, Quota};
use rocket::{
    response::{self, Responder},
    Request,
};

mod catcher;

/// Errors for governed requests which implement 
/// [Responder](rocket::response::Responder).
#[derive(Clone, Debug)]
pub enum LimitError {
    /// Any other undefined LimitError
    Error,

    // TODO: Check ratelimit headers DRAFT for publication
    /// Governed request for the next provided seconds.
    /// Provided `Quota` will be used for setting additional
    /// HTTP headers defined by
    /// [draft-ietf-httpapi-ratelimit-headers](https://datatracker.ietf.org/doc/html/draft-ietf-httpapi-ratelimit-headers).
    /// These headers could be used in service clients to use the service
    /// in a more compliant way for its resources.
    GovernedRequest(u64, Quota),

    /// There is no remote client IP address known in the request. Might be 
    /// a misconfigured server environment.
    MissingClientIpAddr,

    /// Route is not available which might be only the case in fairings
    MissingRoute,

    /// There is a route without name and this can not be matched for 
    /// rate limiting
    MissingRouteName,
}

/// Implements [Responder](rocket::response::Responder) to provide 
/// [Result](rocket::response::Result) possibilities.
impl<'r, 'o: 'r> Responder<'r, 'o> for &LimitError {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'o> {
        let mut handler = catcher::too_many_requests_handler(request);

        match self {
            LimitError::Error => {
                handler.set_header(Header::XRateLimitError("rate limiter error"));
            }
            LimitError::GovernedRequest(wait_time, quota) => {
                handler.set_header(Header::RetryAfter(*wait_time));
                // TODO: x-ratelimit-limit can describe the time window of limit
                //       https://datatracker.ietf.org/doc/html/draft-ietf-httpapi-ratelimit-headers#section-5.1
                handler.set_header(Header::XRateLimitLimit(quota.burst_size().get() as u64));
                // XRateLimitRemaining makes no sense here in LimitError
                // because `state.remaining_burst_capacity()` should be 
                // always 0 
                handler.set_header(Header::XRateLimitReset(
                    quota.burst_size_replenished_in().as_secs(),
                ));
            }
            LimitError::MissingClientIpAddr => {
                handler.set_header(Header::XRateLimitError(
                    "application not retrieving client ip",
                ));
            }
            LimitError::MissingRoute => {
                handler.set_header(Header::XRateLimitError("routing failure"));
            }
            LimitError::MissingRouteName => {
                handler.set_header(Header::XRateLimitError("route without name"));
            }
        };

        Ok(handler)
    }
}
