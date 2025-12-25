#![cfg_attr(not(feature = "perf"), no_std)]

pub mod macros;

#[cfg(feature = "perf")]
pub mod perf;

#[doc(hidden)]
pub use paste;
