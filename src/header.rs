//! The headers used when a [super::RocketGovernor] guarded path responds with TooManyRequests.

use rocket::http;

/// HTTP headers used for rate-limiting.
pub enum Header {
    /// Standard header for status 429 Too Many Requests ([RFC 6585](https://tools.ietf.org/html/rfc6585#section-4)).
    /// This should indicate a client for how long it should wait in seconds for retry.
    RetryAfter(u64),

    /// Custom header for reporting problems with rate limiter.
    XRateLimitError(&'static str),

    /// Header provides the time in seconds when a request to the route is not rate limited
    /// and the rate limiter bucket is full again.
    XRateLimitReset(u64),
}

/// Standard header for status 429 Too Many Requests ([RFC 6585](https://tools.ietf.org/html/rfc6585#section-4)).
/// This should indicate a client for how long it should wait in seconds for retry.
const RETRY_AFTER: &str = "retry-after";

/// Custom header for reporting problems with rate limiter.
const X_RATELIMIT_ERROR: &str = "x-ratelimit-error";

// TODO: https://github.com/kolbma/rocket-governor/issues/2

/// Header provides information about limitation of the route.
// pub const X_RATELIMIT_LIMIT: &str = "x-ratelimit-limit";

/// Header provides information about how many requests are left for the endpoint.
// const X_RATELIMIT_REMAINING: &str = "x-ratelimit-remaining";

/// Header provides the time in seconds when a request to the route is not rate limited
/// and the rate limiter bucket is full again.
const X_RATELIMIT_RESET: &str = "x-ratelimit-reset";

#[doc(hidden)]
impl From<Header> for http::Header<'_> {
    fn from(header: Header) -> Self {
        match header {
            Header::RetryAfter(sec) => http::Header::new(RETRY_AFTER, sec.to_string()),
            Header::XRateLimitError(err) => http::Header::new(X_RATELIMIT_ERROR, err),
            Header::XRateLimitReset(sec) => http::Header::new(X_RATELIMIT_RESET, sec.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Header;
    use super::{RETRY_AFTER, X_RATELIMIT_ERROR, X_RATELIMIT_RESET};
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

        let h: http::Header = Header::XRateLimitReset(5).into();
        assert_eq!(X_RATELIMIT_RESET, h.name());
        assert_eq!(5, u64::from_str(h.value()).unwrap());
    }
}
