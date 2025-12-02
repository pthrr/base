#![feature(decl_macro)]

pub mod macros;
pub mod perf;

// Re-export paste crate for macro expansion
#[doc(hidden)]
pub use paste;
