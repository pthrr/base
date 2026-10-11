#![no_std]

#[cfg(creusot)]
pub mod rtrb_contract;

// Verify production modules against the trusted rtrb call contracts.
#[path = "../../../src/channel.rs"]
mod channel;
#[path = "../../../src/data_input.rs"]
mod data_input;
#[path = "../../../src/event_data_input.rs"]
mod event_data_input;
#[path = "../../../src/event_input.rs"]
mod event_input;

#[cfg(creusot)]
pub use channel::{channel, Receiver, Sender};
pub use data_input::{DataInput, FrozenData};
pub use event_data_input::{
    DispatchInProgress, DropNewest, DropOldest, EventDataInput, FrozenInput, Overflow,
};
pub use event_input::EventInput;
