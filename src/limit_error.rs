//! Errors for governed requests which implement [Responder](rocket::response::Responder).

use super::header::Header;
use rocket::{
    response::{self, Responder},
    Request,
};

mod catcher;

/// Errors for governed requests which implement [Responder](rocket::response::Responder).
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

/// Implements [Responder](rocket::response::Responder) to provide [Result](rocket::response::Result) possibilities.
impl<'r, 'o: 'r> Responder<'r, 'o> for &LimitError {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'o> {
        let mut handler = catcher::too_many_requests_handler(request);

        match self {
            LimitError::Error => {
                handler.set_header(Header::XRateLimitError("rate limiter error"));
            }
            LimitError::GovernedRequest(wait_time) => {
                handler.set_header(Header::RetryAfter(*wait_time));
                handler.set_header(Header::XRateLimitReset(*wait_time));
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
