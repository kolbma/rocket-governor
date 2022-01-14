//! TODO

use crate::{header::Header, ReqState};
use rocket::{
    fairing::{Fairing, Info, Kind},
    Request, Response,
};

/// TODO
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

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let state_opt = ReqState::get_or_default(&request);

        // Set rate limit headers if `RocketGovernable::limit_info_allow()` returns true
        // and `ReqState` will become set `Request.local_cache`.
        if let Some(state) = state_opt {
            response.set_header(Header::XRateLimitLimit(
                state.quota.burst_size().get().into(),
            ));
            response.set_header(Header::XRateLimitRemaining(state.request_capacity.into()));
        }
    }
}
