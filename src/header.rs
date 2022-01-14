//! The headers used when a [RocketGovernor](super::RocketGovernor) guarded 
//! path responds with [`TooManyRequests`](http::Status::TooManyRequests).  
//! 
//! Depending on setup some headers are also set on responding successful
//! to let the client know about request limitations in the near future.
//! 
//! There is an [RFC Draft](https://datatracker.ietf.org/doc/html/draft-ietf-httpapi-ratelimit-headers).
// TODO: Check RFC Draft for publication

use rocket::http;

/// HTTP headers used for rate-limiting.
pub enum Header {
    /// Standard header for status 429 Too Many Requests 
    /// ([RFC 6585](https://tools.ietf.org/html/rfc6585#section-4)).
    /// This should indicate a client for how long it should wait in seconds 
    /// for retry.
    RetryAfter(u64),

    /// Custom header for reporting problems with rate limiter.
    XRateLimitError(&'static str),

    /// Header provides information about limitation of the route.
    XRateLimitLimit(u64),

    /// Header provides information about how many requests are left for the 
    /// endpoint.
    XRateLimitRemaining(u64),

    /// Header provides the time in seconds when a request to the route is not 
    /// rate limited and the rate limiter bucket is full again.
    XRateLimitReset(u64),
}

/// Standard header for status 429 Too Many Requests 
/// ([RFC 6585](https://tools.ietf.org/html/rfc6585#section-4)).
/// This should indicate a client for how long it should wait in seconds for 
/// retry.
pub const RETRY_AFTER: &str = "retry-after";

/// Custom header for reporting problems with rate limiter.
pub const X_RATELIMIT_ERROR: &str = "x-ratelimit-error";

// TODO: https://github.com/kolbma/rocket-governor/issues/2

// TODO: Check ratelimit-headers draft for publication
// https://datatracker.ietf.org/doc/html/draft-ietf-httpapi-ratelimit-headers

/// Header provides information about limitation of the route.
pub const X_RATELIMIT_LIMIT: &str = "x-ratelimit-limit";

/// Header provides information about how many requests are left for the 
/// endpoint.
pub const X_RATELIMIT_REMAINING: &str = "x-ratelimit-remaining";

/// Header provides the time in seconds when a request to the route is not 
/// rate limited and the rate limiter bucket is full again.
pub const X_RATELIMIT_RESET: &str = "x-ratelimit-reset";

#[doc(hidden)]
impl From<Header> for http::Header<'_> {
    fn from(header: Header) -> Self {
        match header {
            Header::RetryAfter(sec) => http::Header::new(RETRY_AFTER, sec.to_string()),
            Header::XRateLimitError(err) => http::Header::new(X_RATELIMIT_ERROR, err),
            Header::XRateLimitLimit(limit) => {
                http::Header::new(X_RATELIMIT_LIMIT, limit.to_string())
            }
            Header::XRateLimitRemaining(remaining) => {
                http::Header::new(X_RATELIMIT_REMAINING, remaining.to_string())
            }
            Header::XRateLimitReset(sec) => http::Header::new(X_RATELIMIT_RESET, sec.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Header;
    use super::{
        RETRY_AFTER, X_RATELIMIT_ERROR, X_RATELIMIT_LIMIT, X_RATELIMIT_REMAINING, X_RATELIMIT_RESET,
    };
    use rocket::http;
    use std::str::FromStr;

    #[test]
    fn test_header() {
        let h: http::Header = Header::RetryAfter(10).into();
        assert_eq!(RETRY_AFTER, h.name());
        assert_eq!(10, u64::from_str(h.value()).unwrap());

        let h: http::Header = Header::XRateLimitError("some error").into();
        assert_eq!(X_RATELIMIT_ERROR, h.name());
        assert_eq!("some error", h.value());

        let h: http::Header = Header::XRateLimitLimit(100).into();
        assert_eq!(X_RATELIMIT_LIMIT, h.name());
        assert_eq!(100, u64::from_str(h.value()).unwrap());

        let h: http::Header = Header::XRateLimitRemaining(1).into();
        assert_eq!(X_RATELIMIT_REMAINING, h.name());
        assert_eq!(1, u64::from_str(h.value()).unwrap());

        let h: http::Header = Header::XRateLimitReset(5).into();
        assert_eq!(X_RATELIMIT_RESET, h.name());
        assert_eq!(5, u64::from_str(h.value()).unwrap());
    }
}
