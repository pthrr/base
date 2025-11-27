#![no_std]

mod channel;
mod data_input;
mod event_data_input;
mod event_input;

pub use channel::{channel, Receiver, Sender};
pub use data_input::{DataInput, FrozenData};
pub use event_data_input::{
    DispatchInProgress, DropNewest, DropOldest, EventDataInput, FrozenInput, Overflow,
};
pub use event_input::EventInput;
