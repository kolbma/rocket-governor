//! Module for trait [RocketGovernable].

use super::{Method, NonZeroU32, Quota};
#[cfg(feature = "limit_info")]
use super::ReqState;
use rocket::async_trait;

/// The [RocketGovernable] guard trait.
///
/// [rocket-governor](crate) is a [rocket] guard implementation of the
/// [governor] rate limiter.
///
/// Declare a struct and use it with the generic
/// [RocketGovernor](super::RocketGovernor) guard.
/// This requires to implement [RocketGovernable] for your struct.
///
/// See the top level [crate] documentation.
///
/// [governor]: https://docs.rs/governor/
/// [rocket]: https://docs.rs/rocket/
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

    /// Returns `true` if HTTP rate limit info [headers](crate::header)
    /// should be set in requests.
    ///
    /// Implement [`limit_info_allow()`](RocketGovernable::limit_info_allow())
    /// to change to your preference.  
    ///
    /// The trait implementation enables info headers only just the request
    /// before any further request would be rate limited.
    /// This is because of **speed**, **bandwidth** and **safety**.
    ///
    /// In
    /// [draft-ietf-httpapi-ratelimit-headers#section-6.2](https://datatracker.ietf.org/doc/html/draft-ietf-httpapi-ratelimit-headers#section-6.2)
    /// you can read the following information about...
    ///
    /// ## Information disclosure
    ///
    /// Servers should not disclose to untrusted parties operational capacity
    /// information that can be used to saturate its infrastructural
    /// resources.
    ///
    /// While this specification does not mandate whether non 2xx responses
    /// consume quota, if 401 and 403 responses count on quota a malicious
    /// client could probe the endpoint to get traffic information of another
    /// user.
    ///
    /// As intermediaries might retransmit requests and consume quota-units
    /// without prior knowledge of the User Agent, RateLimit fields might
    /// reveal the existence of an intermediary to the User Agent.
    ///
    /// ## Feature availability
    ///
    /// [`limit_info_allow()`](RocketGovernable::limit_info_allow()) is only
    /// available when feature __limit_info__ is enabled.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use rocket_governor::{Method, Quota, ReqState, RocketGovernable};
    ///
    /// pub struct RateLimitGuard;
    ///
    /// impl<'r> RocketGovernable<'r> for RateLimitGuard {
    ///     fn quota(_method: Method, _route_name: &str) -> Quota {
    ///         Quota::per_second(Self::nonzero(1u32))
    ///     }
    ///
    ///     fn limit_info_allow(
    ///         method: Option<Method>,
    ///         route_name: Option<&str>,
    ///         state: &ReqState,
    ///     ) -> bool {
    ///         let mut cap = 1;
    ///         if let Some(m) = method {
    ///             if m == Method::Post {
    ///                 if let Some(route) = route_name {
    ///                     if route == "admin_action" {
    ///                         cap = 20;
    ///                     }
    ///                 }
    ///             }
    ///         }
    ///         state.request_capacity() <= cap
    ///     }
    /// }
    ///
    /// ```
    ///
    // TODO: update RFC link when published
    #[cfg(feature = "limit_info")]
    fn limit_info_allow(
        method: Option<Method>,
        route_name: Option<&str>,
        state: &ReqState,
    ) -> bool {
        let (_, _) = (method, route_name); // unused warning

        state.request_capacity <= 1
    }

    /// Converts a non-zero number [u32] to [NonZeroU32](std::num::NonZeroU32).
    ///
    /// Number zero/0 becomes 1.
    ///
    #[inline]
    fn nonzero(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap_or_else(|| NonZeroU32::new(1u32).unwrap())
    }
}

#[cfg(feature = "limit_info")]
#[cfg(test)]
mod tests {
    use super::*;
    use rocket::{
        get,
        http::{Header, Status},
        local::blocking::Client,
        routes, Build, Rocket,
    };

    struct RateLimitGuard;

    impl<'r> RocketGovernable<'r> for RateLimitGuard {
        fn quota(_method: Method, _route_name: &str) -> Quota {
            Quota::per_second(Self::nonzero(1u32))
        }
    }

    #[get("/")]
    fn route_test() -> Status {
        Status::Ok
    }

    fn launch_rocket() -> Rocket<Build> {
        rocket::build().mount("/", routes![route_test])
    }

    #[test]
    fn test_limit_info_allow() {
        let client = Client::untracked(launch_rocket()).expect("no rocket instance");
        let mut req = client.get("/");
        req.add_header(Header::new("X-Real-IP", "127.2.1.1"));
        // req.dispatch();
        let request = req.inner_mut();
        let state = request
            .local_cache(|| ReqState::new(Quota::per_second(NonZeroU32::new(1).unwrap()), 2));
        let _ = request.real_ip();

        assert!(!RateLimitGuard::limit_info_allow(None, None, state));

        let mut req = client.get("/");
        req.add_header(Header::new("X-Real-IP", "127.2.1.2"));
        // req.dispatch();
        let request = req.inner_mut();
        let state = request
            .local_cache(|| ReqState::new(Quota::per_second(NonZeroU32::new(1).unwrap()), 1));
        let _ = request.real_ip();

        assert!(RateLimitGuard::limit_info_allow(None, None, state));

        let mut req = client.get("/");
        req.add_header(Header::new("X-Real-IP", "127.2.1.3"));
        // req.dispatch();
        let request = req.inner_mut();
        let state = request
            .local_cache(|| ReqState::new(Quota::per_second(NonZeroU32::new(1).unwrap()), 0));
        let _ = request.real_ip();

        assert!(RateLimitGuard::limit_info_allow(None, None, state));
    }
}
