//! Errors for governed requests which implement [Responder].
//!
//! [Responder]: https://api.rocket.rs/v0.5-rc/rocket/response/responder/trait.Responder.html

use super::header::Header;
use rocket::{
    http::Status,
    response::{self, Responder},
    Request, Response,
};

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

#[doc(hidden)]
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
