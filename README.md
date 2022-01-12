[![Rust](https://github.com/kolbma/rocket-governor/actions/workflows/rust.yml/badge.svg)](https://github.com/kolbma/rocket-governor/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/rocket-governor)](https://crates.io/crates/rocket-governor)
[![docs](https://docs.rs/rocket-governor/badge.svg)](https://docs.rs/rocket-governor)

## Description

Implementation of the [Governor](https://github.com/antifuchs/governor.git) rate limiter for [Rocket](https://rocket.rs) web framework.

Rate limiting is used to control the rate requests are received and handled by an endpoint of the web application 
or web service.

## Rocket specific features

Define as many rate limits with [Quota](https://docs.rs/governor/latest/governor/struct.Quota.html) of Governor
as you like and need in your Rocket web application/service.  
It is implemented as a Rocket [Request Guard](https://rocket.rs/v0.5-rc/guide/requests/#request-guards) and provides
also an implementation of an [Error Catcher](https://rocket.rs/v0.5-rc/guide/requests/#error-catchers).  
The Error Catcher can be registered on any path to handle [`Status::TooManyRequests`](https://api.rocket.rs/v0.5-rc/rocket/http/struct.Status.html#associatedconstant.TooManyRequests) and provide HTTP headers in the response.

## Usage

Add dependencies to [rocket-governor](https://crates.io/crates/rocket-governor) crate to your _Cargo.toml_.

Implement `RocketGovernable` for a _guard struct_ as you like: 

```rust
use rocket_governor::{Method, Quota, RocketGovernable, RocketGovernor};

pub struct RateLimitGuard;

impl<'r> RocketGovernable<'r> for RateLimitGuard {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::per_second(Self::nonzero(1u32))
    }
}
```

This requires to implement the method `fn quota(_: Method, _: &str) -> Quota`.  
You can vary your `Quota` on any combination of __method__ and __route_name__, but the returned `Quota` should be a _static-like_. It should __not change__ between invocations of the `quota()`-method with equal parameters.

There is a small helper function `nonzero(u32)` for creating Quotas in your `quota()`-implementation e.g.:
```rust
    Quota::per_second(Self::nonzero(1u32))
```

After implementing the minimal requirements of trait `RocketGovernable` you can add your _Guard_ to your route-methods like:

```rust
#[get("/")]
fn route_test(_limitguard: RocketGovernor<RateLimitGuard>) -> Status {
    Status::Ok
}
```

### Register Catcher

To handle HTTP Status 429 TooManyRequests there is an catcher-function implementation.

It is __called__ __`rocket_governor_catcher`__.  

Register it with `register(<path>, <catchers>)`-method of Rocket:

```rust
use rocket_governor::rocket_governor_catcher;

#[launch]
fn launch_rocket() -> _ {
    rocket::build()
        .mount("/", routes![route_test])
        .register("/", catchers!(rocket_governor_catcher))
}
```

### Optional feature __logger__

There is the optional feature __logger__ which enables some logging output.

For usage depend on it in Cargo.toml
```toml
[dependencies]
rocket-governor = { version = "...", features = ["logger"] }
```

### Additional information

To understand the basics of Rocket, please visit the _Rocket Guide_:
* https://rocket.rs/v0.5-rc/guide/requests/#request-guards
* https://rocket.rs/v0.5-rc/guide/requests/#error-catchers

## Licenses

You can choose between __[MIT License](https://opensource.org/licenses/MIT)__ or __[Apache License 2.0](http://www.apache.org/licenses/LICENSE-2.0)__.

### MIT License

Copyright (c) 2022 Markus Kolb

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice (including the next paragraph) shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

### Apache License 2.0

Copyright 2022 Markus Kolb

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
