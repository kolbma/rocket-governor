## Description

Implementation of the [Governor](https://github.com/antifuchs/governor.git) rate limiter for [Rocket](https://rocket.rs) web framework.

Rate limiting is used to control the rate requests are received and handled by an endpoint of the web application 
or web service.

## Rocket specific features

Define as many rate limits with [Quota](https://docs.rs/governor/latest/governor/struct.Quota.html) of Governor
as you like or need in your Rocket web application/service.  
It is implemented as a Rocket [Request Guard](https://rocket.rs/v0.5-rc/guide/requests/#request-guards) and provides
also an implementation of an [Error Catcher](https://rocket.rs/v0.5-rc/guide/requests/#error-catchers).  
The Error Catcher can be registered on any path to handle [`Status::TooManyRequests`](https://api.rocket.rs/v0.5-rc/rocket/http/struct.Status.html#associatedconstant.TooManyRequests) and providing HTTP headers in the response.

## Usage

Add dependencies to [rocket-governor](https://crates.io/crates/rocket-governor) and [rocket-governor-derive](https://crates.io/crates/rocket-governor-derive) crates to your _Cargo.toml_.

Implement `RocketGovernable` for a _guard struct_ which derives from `RocketGovernor`: 

```rust
use rocket_governor::{Method, Quota, RocketGovernable};
use rocket_governor_derive::{RocketGovernor, RocketGovernorWithMember};


#[derive(RocketGovernor)]
pub struct RateLimitGuard;

impl<'r> RocketGovernable<'r> for RateLimitGuard {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::per_second(Self::nonzero(1u32))
    }
}
```

This requires to implement the `fn quota(_: Method, _: &str) -> Quota`.  
You can vary your `Quota` on any combination of __method__ and __route_name__, but the returned `Quota` should be a _static-like_. It should __not change__ between invocations of the `quota()`-method.

If your struct has members, there is an alternative `derive` which requires to implement [`Default`](https://doc.rust-lang.org/std/default/trait.Default.html):

```rust
#[derive(RocketGovernorWithMember)]
pub struct RateLimitGuardWithMember {
    pub member: u8,
}

impl<'r> RocketGovernable<'r> for RateLimitGuardWithMember {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::with_period(Duration::from_secs(2u64)).unwrap()
    }
}

impl Default for RateLimitGuardWithMember {
    fn default() -> Self {
        Self { member: 254u8 }
    }
}
```

There is a small helper function `nonzero(u32)` for creating Quotas in your `quota()`-implementation e.g.:
```rust
    Quota::per_second(Self::nonzero(1u32))
```

After this you can add your _Guard_ to your route-methods like:

```rust
#[get("/")]
fn route_test(_limitguard: RateLimitGuard) -> Status {
    Status::Ok
}
```

### Register Catcher

To handle HTTP Status 421 TooManyRequests there is an catcher-function implementation.

It is __called__ like your __*`struct`-name*__ in __lowercase__ with the extension __*_rocket_governor_catcher*__.  
So in this usage example there exist two catcher-functions `ratelimitguard_rocket_governor_catcher()` and `ratelimitguardwithmember_rocket_governor_catcher()`.

Register it with `register(<path>, <catchers>)`-method of Rocket:

```rust
#[launch]
fn launch_rocket() -> _ {
    rocket::build()
        .mount("/", routes![route_test])
        .register("/", catchers!(ratelimitguard_rocket_governor_catcher))
}
```

### Additional information

To understand the basics of Rocket, please have a look in the _Rocket Guide_:
* https://rocket.rs/v0.5-rc/guide/requests/#request-guards
* https://rocket.rs/v0.5-rc/guide/requests/#error-catchers

## Licenses

You can choose between __[MIT License](https://opensource.org/licenses/MIT)__ or __[Apache License 2.0](http://www.apache.org/licenses/LICENSE-2.0)__.

### MIT License

Copyright (c) 2021 Markus Kolb

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice (including the next paragraph) shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

### Apache License 2.0

Copyright 2021 Markus Kolb

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
