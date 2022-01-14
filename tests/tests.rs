#![deny(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::all)]

use rocket::{
    catchers, get,
    http::{Accept, ContentType, Header, Status},
    launch,
    local::blocking::Client,
    routes,
};
use rocket_governor::header as rg_header;
use rocket_governor::{rocket_governor_catcher, Method, Quota, RocketGovernable, RocketGovernor};
use std::{str::FromStr, thread, time::Duration};

pub struct RateLimitGuard;

impl<'r> RocketGovernable<'r> for RateLimitGuard {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::per_second(Self::nonzero(1u32))
    }
}

pub struct RateLimitGuardWithMember {
    pub member: u8,
}

impl<'r> RocketGovernable<'r> for RateLimitGuardWithMember {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::with_period(Duration::from_secs(2u64)).unwrap()
    }
}

pub struct RateLimitGGuard;
impl<'r> RocketGovernable<'r> for RateLimitGGuard {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::per_second(Self::nonzero(1u32))
    }
}

#[get("/")]
fn route_test(_limitguard: RocketGovernor<RateLimitGuard>) -> Status {
    Status::Ok
}

#[get("/member")]
fn route_member(_limitguard: RocketGovernor<RateLimitGuardWithMember>) -> Status {
    Status::Ok
}

mod guard2 {
    use rocket::{get, http::Status};
    use rocket_governor::{Method, Quota, RocketGovernable, RocketGovernor};

    pub struct RateLimitGuard;

    impl<'r> RocketGovernable<'r> for RateLimitGuard {
        fn quota(_method: Method, route_name: &str) -> Quota {
            match route_name {
                "route_hour" => Quota::per_hour(Self::nonzero(1)),
                "route_multi" => Quota::per_hour(Self::nonzero(4)),
                _ => Quota::per_second(Self::nonzero(1u32)),
            }
        }
    }

    #[get("/")]
    pub fn route_test(_limitguard: RocketGovernor<RateLimitGuard>) -> Status {
        Status::Ok
    }

    #[get("/hour")]
    pub fn route_hour(_limitguard: RocketGovernor<RateLimitGuard>) -> Status {
        Status::Ok
    }

    #[get("/multi")]
    pub fn route_multi(_limitguard: RocketGovernor<RateLimitGuard>) -> Status {
        Status::Ok
    }
}

#[launch]
fn launch_rocket() -> _ {
    #[allow(unused_mut)] // attach fairing only on feature limit_info
    let mut r = rocket::build()
        .mount("/", routes![route_test, route_member])
        .register("/", catchers!(rocket_governor_catcher))
        .mount(
            "/guard2",
            routes![guard2::route_test, guard2::route_hour, guard2::route_multi],
        )
        .register("/guard2", catchers!(rocket_governor_catcher));

    #[cfg(feature = "limit_info")]
    {
        r = r.attach(rocket_governor::LimitHeaderGen::default());
    }

    r
}

#[test]
fn test_ratelimit() {
    let client = Client::untracked(launch_rocket()).expect("no rocket instance");
    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.1.1"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.1.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.1.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.1.2"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    thread::sleep(Duration::from_millis(1100u64));

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.1.1"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    thread::sleep(Duration::from_millis(300u64));

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.1.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());
}

#[test]
fn test_ratelimit_with_member() {
    let client = Client::untracked(launch_rocket()).expect("no rocket instance");
    let mut req = client.get("/member");
    req.add_header(Header::new("X-Real-IP", "127.0.2.1"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    let mut req = client.get("/member");
    req.add_header(Header::new("X-Real-IP", "127.0.2.2"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    let mut req = client.get("/member");
    req.add_header(Header::new("X-Real-IP", "127.0.2.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());

    thread::sleep(Duration::from_millis(1100u64));

    let mut req = client.get("/member");
    req.add_header(Header::new("X-Real-IP", "127.0.2.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());

    thread::sleep(Duration::from_millis(900u64));

    let mut req = client.get("/member");
    req.add_header(Header::new("X-Real-IP", "127.0.2.1"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());
}

#[test]
fn test_ratelimit_header() {
    let client = Client::untracked(launch_rocket()).expect("no rocket instance");
    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.3.1"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.3.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());

    let ratelimit_header = res.headers().get_one(rg_header::X_RATELIMIT_ERROR);
    assert_eq!(None, ratelimit_header);

    let reset_header = res.headers().get_one(rg_header::X_RATELIMIT_RESET);
    assert_ne!(None, reset_header);
    let reset_header = reset_header.unwrap();
    assert!(!reset_header.is_empty());
    u64::from_str(reset_header).unwrap();

    let mut req = client.get("/guard2/hour");
    req.add_header(Header::new("X-Real-IP", "127.0.3.2"));
    req.dispatch();
    let mut req = client.get("/guard2/hour");
    req.add_header(Header::new("X-Real-IP", "127.0.3.2"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());

    let retry_header = res.headers().get_one(rg_header::RETRY_AFTER);
    assert_ne!(None, retry_header);
    let retry_header = retry_header.unwrap();
    assert!(!retry_header.is_empty());
    assert!(u64::from_str(retry_header).unwrap() > 59 * 60);
}

#[test]
fn test_ratelimit_body() {
    let client = Client::untracked(launch_rocket()).expect("no rocket instance");
    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.4.1"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.4.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());
    assert_eq!(ContentType::HTML, res.content_type().unwrap());

    let body_string = res.into_string().unwrap();

    assert!(body_string.starts_with("<!DOCTYPE html>"));
    assert!(body_string.contains("429"));

    let req = client.get("/");
    let res = req
        .header(Accept::JSON)
        .header(Header::new("X-Real-IP", "127.0.4.1"))
        .dispatch();

    assert_eq!(Status::TooManyRequests, res.status());
    assert_eq!(ContentType::JSON, res.content_type().unwrap());

    let body_string = res.into_string().unwrap();

    assert!(body_string.starts_with('{'));
    assert!(body_string.contains("\"code\": 429"));
}

#[test]
fn test_ratelimit_guards_are_separated() {
    let client = Client::untracked(launch_rocket()).expect("no rocket instance");
    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.5.1"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    let mut req = client.get("/guard2");
    req.add_header(Header::new("X-Real-IP", "127.0.5.1"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.5.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());

    let mut req = client.get("/guard2");
    req.add_header(Header::new("X-Real-IP", "127.0.5.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());
}

#[cfg(feature = "limit_info")]
#[test]
fn test_ratelimit_info_header() {
    let client = Client::untracked(launch_rocket()).expect("no rocket instance");
    let mut req = client.get("/guard2/multi");
    req.add_header(Header::new("X-Real-IP", "127.0.6.1"));
    let res = req.dispatch();
    assert_eq!(Status::Ok, res.status());
    let mut req = client.get("/guard2/multi");
    req.add_header(Header::new("X-Real-IP", "127.0.6.1"));
    let res = req.dispatch();
    assert_eq!(Status::Ok, res.status());

    let retry_header = res.headers().get_one(rg_header::RETRY_AFTER);
    assert_eq!(None, retry_header);

    let remain_header = res.headers().get_one(rg_header::X_RATELIMIT_REMAINING);
    assert_eq!(None, remain_header); // only set when <= 1 remains

    let mut req = client.get("/guard2/multi");
    req.add_header(Header::new("X-Real-IP", "127.0.6.1"));
    let res = req.dispatch();
    assert_eq!(Status::Ok, res.status());

    let remain_header = res.headers().get_one(rg_header::X_RATELIMIT_REMAINING);
    assert_ne!(None, remain_header); // remains == 1
    let remain_header = remain_header.unwrap();
    assert!(!remain_header.is_empty());
    assert_eq!(u32::from_str(remain_header).unwrap(), 1);

    let mut req = client.get("/guard2/multi");
    req.add_header(Header::new("X-Real-IP", "127.0.6.1"));
    let res = req.dispatch();
    assert_eq!(Status::Ok, res.status());

    let remain_header = res.headers().get_one(rg_header::X_RATELIMIT_REMAINING);
    assert_ne!(None, remain_header); // remains == 0
    let remain_header = remain_header.unwrap();
    assert!(!remain_header.is_empty());
    assert_eq!(u32::from_str(remain_header).unwrap(), 0);

    let mut req = client.get("/guard2/multi");
    req.add_header(Header::new("X-Real-IP", "127.0.6.1"));
    let res = req.dispatch();
    assert_eq!(Status::TooManyRequests, res.status());

    let limit_header = res.headers().get_one(rg_header::X_RATELIMIT_LIMIT);
    assert_eq!(Some("4"), limit_header);
    let remain_header = res.headers().get_one(rg_header::X_RATELIMIT_REMAINING);
    assert_eq!(None, remain_header); // only set on not limited
    let retry_header = res.headers().get_one(rg_header::RETRY_AFTER);
    assert_ne!(None, retry_header);
}
