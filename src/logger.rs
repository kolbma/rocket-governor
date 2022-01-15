#![allow(clippy::single_component_path_imports)] // this is how the feature works

#[cfg(feature = "logger")]
pub(crate) use rocket::{debug, error, info, trace};

#[cfg(not(feature = "logger"))]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)+) => {};
    ($($arg:tt)+) => {};
}
#[cfg(not(feature = "logger"))]
pub(crate) use debug;

#[cfg(not(feature = "logger"))]
macro_rules! error {
    (target: $target:expr, $($arg:tt)+) => {};
    ($($arg:tt)+) => {};
}
#[cfg(not(feature = "logger"))]
pub(crate) use error;

#[cfg(not(feature = "logger"))]
macro_rules! info {
    (target: $target:expr, $($arg:tt)+) => {};
    ($($arg:tt)+) => {};
}
#[cfg(not(feature = "logger"))]
pub(crate) use info;

#[cfg(not(feature = "logger"))]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)+) => {};
    ($($arg:tt)+) => {};
}
#[cfg(not(feature = "logger"))]
pub(crate) use trace;
