//! Receiving adapter for an AADL data input port on a sampled connection.
//!
//! A [`DataInput`] is an [`EventDataInput`] with `DropOldest` and queue size 1
//! that keeps the last value it froze. Each dispatch therefore reads the
//! latest value sent before its input time, and a dispatch without a new send
//! reads the previous value again. This is the Rust counterpart of
//! `RTCHAN_DATA_INPUT` in `models/data_input.m4`.
//!
//! ```
//! use rtchan::DataInput;
//!
//! let (mut tx, mut input) = DataInput::<u8>::new(2);
//! assert_eq!(input.dispatch().get(), None);
//! tx.try_send(7).unwrap();
//! let frozen = input.dispatch();
//! assert_eq!((frozen.get(), frozen.is_updated()), (Some(&7), true));
//! let again = input.dispatch();
//! assert_eq!((again.get(), again.is_updated()), (Some(&7), false));
//! ```

use crate::{DropOldest, EventDataInput, Sender};

#[cfg(creusot)]
use crate::event_data_input::DispatchObservation;

#[cfg(creusot)]
use creusot_std::prelude::{ensures, logic, requires, Invariant};

/// Input adapter for one AADL data input port.
///
/// It owns the receiving end of its channel; use one adapter per input
/// connection.
pub struct DataInput<T> {
    input: EventDataInput<T, DropOldest>,
    latest: Option<T>,
}

#[cfg(creusot)]
impl<T> Invariant for DataInput<T> {
    #[logic]
    fn invariant(self) -> bool {
        pearlite! { !self.input.active && self.input.queue_size == 1usize }
    }
}

/// The value frozen for one dispatch.
pub struct FrozenData<'a, T> {
    value: Option<&'a T>,
    updated: bool,
    #[cfg(creusot)]
    observation: DispatchObservation<T>,
}

impl<T> DataInput<T> {
    /// Allocates a data port connection.
    ///
    /// `max_sends` must bound the sends from the creation of the connection
    /// to the return of the first dispatch, and from the start of each
    /// dispatch to the return of the next; the channel gets that capacity. A
    /// full send means the bound was violated, and the input may then read a
    /// value older than the latest.
    ///
    /// # Panics
    ///
    /// Panics if `max_sends` is zero.
    #[cfg_attr(creusot, requires(max_sends > 0usize))]
    #[cfg_attr(creusot, ensures(result.1.latest == None))]
    #[cfg_attr(creusot, ensures(result.0.capacity() == max_sends@))]
    pub fn new(max_sends: usize) -> (Sender<T>, Self) {
        let (sender, input) = EventDataInput::<T, DropOldest>::new(1, max_sends);
        let input = Self {
            input,
            latest: None,
        };
        (sender, input)
    }

    /// Freezes the latest value sent before the input time.
    ///
    /// Align its length read with the AADL input time. The result borrows the
    /// input, so no later send changes it. Never waits or allocates; drops at most
    /// `max_sends` older values in place, running their destructors.
    #[cfg_attr(
        creusot,
        ensures(match &(^self).latest {
            Some(value) => result.value == Some(value),
            None => result.value == None,
        })
    )]
    #[cfg_attr(creusot, ensures(!result.updated ==> (^self).latest == (*self).latest))]
    #[cfg_attr(
        creusot,
        ensures(result.updated ==
            (result.observation.observed().len() > result.observation.start_index()))
    )]
    #[cfg_attr(
        creusot,
        ensures(result.observation.end_index() ==
            if result.updated { result.observation.selected_index() + 1 }
            else { result.observation.start_index() })
    )]
    #[cfg_attr(
        creusot,
        ensures(result.observation.selected_index() == result.observation.start_index()
            + if result.observation.observed().len() > result.observation.start_index() + 1 {
                result.observation.observed().len() - result.observation.start_index() - 1
            } else { 0 })
    )]
    #[cfg_attr(
        creusot,
        ensures(result.observation.observed().len() > result.observation.start_index() ==>
            result.updated && match result.value {
                Some(value) => *value == result.observation.observed()[result.observation.selected_index()],
                None => false,
            })
    )]
    #[cfg_attr(
        creusot,
        ensures(result.updated ==> match result.value {
            Some(value) => result.observation.selected_index() < result.observation.received().len()
                && *value == result.observation.received()[result.observation.selected_index()],
            None => false,
        })
    )]
    pub fn dispatch(&mut self) -> FrozenData<'_, T> {
        // The inner adapter freezes and completes this dispatch in one call.
        let (fresh, _observation) = self.input.dispatch_value();
        let updated = update_latest(&mut self.latest, fresh);
        FrozenData {
            value: self.latest.as_ref(),
            updated,
            #[cfg(creusot)]
            observation: _observation,
        }
    }

    /// Returns whether the sender has been dropped.
    pub fn is_abandoned(&self) -> bool {
        self.input.is_abandoned()
    }
}

#[cfg_attr(
    creusot,
    ensures(match fresh {
        Some(value) => result && ^latest == Some(value),
        None => !result && ^latest == *latest,
    })
)]
fn update_latest<T>(latest: &mut Option<T>, fresh: Option<T>) -> bool {
    match fresh {
        Some(value) => {
            *latest = Some(value);
            true
        }
        None => false,
    }
}

impl<'a, T> FrozenData<'a, T> {
    /// Returns the frozen value, or `None` before the first send.
    ///
    /// Corresponds to AADL `Get_Value`.
    #[cfg_attr(creusot, ensures(result == self.value))]
    pub fn get(&self) -> Option<&'a T> {
        self.value
    }

    /// Returns whether a send arrived since the previous dispatch.
    ///
    /// Corresponds to AADL `Updated`.
    #[cfg_attr(creusot, ensures(result == self.updated))]
    pub fn is_updated(&self) -> bool {
        self.updated
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::DataInput;

    #[test]
    fn rereads_latest_value_until_next_send() {
        let (mut tx, mut input) = DataInput::<u8>::new(3);
        tx.try_send(1).unwrap();
        tx.try_send(2).unwrap();
        let frozen = input.dispatch();
        tx.try_send(3).unwrap();
        assert_eq!((frozen.get(), frozen.is_updated()), (Some(&2), true));

        let next = input.dispatch();
        assert_eq!((next.get(), next.is_updated()), (Some(&3), true));
        let again = input.dispatch();
        assert_eq!((again.get(), again.is_updated()), (Some(&3), false));
    }

    #[test]
    fn empty_until_first_send() {
        let (_tx, mut input) = DataInput::<u8>::new(1);
        let frozen = input.dispatch();
        assert_eq!((frozen.get(), frozen.is_updated()), (None, false));
    }

    #[test]
    #[should_panic(expected = "send bound must be positive")]
    fn rejects_zero_send_bound() {
        let _ = DataInput::<u8>::new(0);
    }

    #[test]
    fn matches_latest_value_register_on_bounded_traces() {
        use std::vec::Vec;

        #[derive(Clone, Copy)]
        enum Op {
            Send(u8),
            Dispatch,
        }

        fn check(max_sends: usize, ops: &[Op]) {
            let (mut tx, mut input) = DataInput::<u8>::new(max_sends);
            let (mut latest, mut fresh) = (None, false);
            for op in ops {
                match *op {
                    Op::Send(message) => {
                        assert_eq!(tx.try_send(message), Ok(()));
                        (latest, fresh) = (Some(message), true);
                    }
                    Op::Dispatch => {
                        let frozen = input.dispatch();
                        assert_eq!(frozen.get().copied(), latest);
                        assert_eq!(frozen.is_updated(), fresh);
                        fresh = false;
                    }
                }
            }
        }

        // Dispatch is atomic in one thread, so `sends` counts the window
        // since the previous dispatch.
        fn explore(k: usize, remaining: usize, sends: usize, ops: &mut Vec<Op>) {
            check(k, ops);
            if remaining == 0 {
                return;
            }
            if sends < k {
                for message in [0, 1, 2] {
                    ops.push(Op::Send(message));
                    explore(k, remaining - 1, sends + 1, ops);
                    ops.pop();
                }
            }
            ops.push(Op::Dispatch);
            explore(k, remaining - 1, 0, ops);
            ops.pop();
        }

        for max_sends in [1, 2, 3] {
            explore(max_sends, 7, 0, &mut Vec::new());
        }
    }

    #[test]
    fn threads_see_monotonic_latest_values() {
        let (mut tx, mut input) = DataInput::<u32>::new(4);
        let writer = std::thread::spawn(move || {
            for message in 0..10_000 {
                let mut value = message;
                while let Err(back) = tx.try_send(value) {
                    value = back;
                    std::thread::yield_now();
                }
            }
        });
        let mut previous = None;
        while previous != Some(9_999) {
            let frozen = input.dispatch();
            let value = frozen.get().copied();
            if frozen.is_updated() {
                assert!(value > previous, "stale value marked updated");
            } else {
                assert_eq!(value, previous, "value changed without update");
            }
            previous = value;
        }
        writer.join().unwrap();
    }
}
