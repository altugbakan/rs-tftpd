#![allow(unused_imports)]

use std::cmp::max;
use std::sync::OnceLock;

static VERBOSITY: OnceLock<usize> = OnceLock::new();

/// Verbosity should be set once at program start.
pub fn verbosity_set(verbosity : isize) {
    VERBOSITY.get_or_init(|| max(0, verbosity) as usize);
}

/// Helper function to retrieve verbosity level for following macros
pub fn verbosity() -> usize {
    *VERBOSITY.get().unwrap_or(&1)
}

/// Report error logs
#[macro_export]
macro_rules! log_err {
    ($($x:tt)*) => { eprintln!($($x)*) }
}

/// Report warning logs
#[macro_export]
macro_rules! log_warn {
    ($($x:tt)*) => { if  0 < $crate::verbosity() { println!($($x)*)} }
}

/// Report info logs
#[macro_export]
macro_rules! log_info {
    ($($x:tt)*) => { if  1 < $crate::verbosity() { println!($($x)*)} }
}

/// Report debug logs
#[macro_export]
#[cfg(debug_assertions)]
macro_rules! log_dbg {
    ($($x:tt)*) => { if  2 < $crate::verbosity() { println!($($x)*)} }
}

/// Do not compile debug logs with release target
#[macro_export]
#[cfg(not(debug_assertions))]
macro_rules! log_dbg {
    ($($x:tt)*) => { () }
}

pub(crate) use log_err;
pub(crate) use log_warn;
pub(crate) use log_info;
pub(crate) use log_dbg;
