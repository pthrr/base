#![no_std]

//! Shared error, macro, and channel reexports.
//!
//! ```
//! let _: base::error::Error = base::error::Error::Value("invalid");
//! assert_eq!(base::invoke!({ 2 + 3 }), 5);
//! let (mut tx, mut rx) = base::rtchan::channel::<u8>(1);
//! tx.try_send(7).unwrap();
//! assert_eq!(rx.try_recv(), Some(7));
//! let (mut tx, mut event) = base::rtchan::EventInput::<base::rtchan::DropNewest>::new(1);
//! tx.try_send(()).unwrap();
//! assert_eq!(event.dispatch().unwrap().complete(), Some(()));
//! ```

pub use base_error as error;
pub use base_macros as macros;
pub use base_macros::invoke;
pub use rtchan;
