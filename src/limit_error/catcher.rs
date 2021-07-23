//! Provides a similiar catcher for html/json like the Rocket framework

use rocket::{
    http::{ContentType, Status},
    Request, Response,
};
use std::{borrow::Cow, io::Cursor};

// copied from rocket/src/catcher/catcher.rs because it is also pub(crate)

macro_rules! html_error_template {
    ($code:expr, $reason:expr, $description:expr) => {
        concat!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <title>"#,
            $code,
            " ",
            $reason,
            r#"</title>
</head>
<body align="center">
    <div role="main" align="center">
        <h1>"#,
            $code,
            ": ",
            $reason,
            r#"</h1>
        <p>"#,
            $description,
            r#"</p>
        <hr />
    </div>
    <div role="contentinfo" align="center">
        <small>Rocket</small>
    </div>
</body>
</html>"#
        )
    };
}

macro_rules! json_error_template {
    ($code:expr, $reason:expr, $description:expr) => {
        concat!(
            r#"{
  "error": {
    "code": "#,
            $code,
            r#",
    "reason": ""#,
            $reason,
            r#"",
    "description": ""#,
            $description,
            r#""
  }
}"#
        )
    };
}

// also copied and modified

/// Create the handler for `Status::TooManyRequests`.
pub(crate) fn too_many_requests_handler<'r, 'o: 'r>(req: &'r Request<'_>) -> Response<'o> {
    let preferred = req.accept().map(|a| a.preferred());
    let (mime, text) = if preferred.map_or(false, |a| a.is_json()) {
        let json: Cow<'_, str> = json_error_template!(
            429,
            "Too Many Requests",
            "Too many requests have been received recently."
        )
        .into();

        (ContentType::JSON, json)
    } else {
        let html: Cow<'_, str> = html_error_template!(
            429,
            "Too Many Requests",
            "Too many requests have been received recently."
        )
        .into();

        (ContentType::HTML, html)
    };

    let mut r = Response::build()
        .status(Status::TooManyRequests)
        .header(mime)
        .finalize();
    match text {
        Cow::Owned(v) => r.set_sized_body(v.len(), Cursor::new(v)),
        Cow::Borrowed(v) => r.set_sized_body(v.len(), Cursor::new(v)),
    };

    r
}
