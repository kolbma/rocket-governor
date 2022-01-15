//! Provides [`Fairing`](rocket::fairing::Fairing) in the implementation
//! [`LimitHeaderGen`] which is [attachable](rocket::Rocket::attach()) to
//! [`Rocket`](rocket::Rocket)-instance.

use crate::{header::Header, ReqState};
use rocket::{
    fairing::{Fairing, Info, Kind},
    Request, Response,
};

/// Provides [`Fairing`](rocket::fairing::Fairing) implementation
/// which is [attachable](rocket::Rocket::attach()) to
/// [`Rocket`](rocket::Rocket)-instance.
///
/// `LimitHeaderGen` generates [HTTP headers](crate::header) in
/// [Reponses](rocket::Response) to [Requests](rocket::Request) at specific
/// [mounted routes](rocket::Rocket::mount()).
///
/// The [`Route`](rocket::Route) needs a [RocketGovernor guard](crate).
/// The guard is responsible for managing the governor rate limit.
///
/// Depending on the implementation of
/// [`RocketGovernable::limit_info_allow()`](crate::RocketGovernable::limit_info_allow()),
/// `LimitHeaderGen` fairing sets [HTTP headers](crate::header) like
/// 
/// * [X_RATELIMIT_LIMIT](crate::header::X_RATELIMIT_LIMIT)
/// * [X_RATELIMIT_REMAINING](crate::header::X_RATELIMIT_REMAINING)
/// 
/// which can be used by HTTP clients to adjust service requests.
///
/// ## Example usage
///
/// ```rust
/// use rocket;
/// use rocket_governor;
///
/// #[rocket::launch]
/// fn launch_rocket() -> _ {
///     rocket::build().attach(rocket_governor::LimitHeaderGen::default())
/// }
/// ```
///
pub struct LimitHeaderGen;

impl Default for LimitHeaderGen {
    fn default() -> Self {
        Self
    }
}

#[rocket::async_trait]
impl Fairing for LimitHeaderGen {
    fn info(&self) -> Info {
        Info {
            name: "RateLimit Header Generator",
            kind: Kind::Response,
        }
    }

    /// Set rate limit headers if
    /// [`RocketGovernable::limit_info_allow()`](crate::RocketGovernable::limit_info_allow())
    /// returns true and [`ReqState`] has been cached in this case in
    /// [`Request::local_cache`].
    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let state_opt = ReqState::get_or_default(&request);

        if let Some(state) = state_opt {
            response.set_header(Header::XRateLimitLimit(
                state.quota.burst_size().get().into(),
            ));
            response.set_header(Header::XRateLimitRemaining(state.request_capacity.into()));
        }
    }
}
