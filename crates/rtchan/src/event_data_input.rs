//! Receiving adapter for an AADL event data input port.
//!
//! This is the Rust counterpart of `RTCHAN_EVENT_DATA_INPUT` in
//! `models/event_data_input.m4`. An [`EventDataInput`] owns the receiving end of its
//! channel, and its type parameter is the AADL overflow policy: [`DropNewest`]
//! or [`DropOldest`]. The caller invokes [`EventDataInput::dispatch`] at its
//! configured input time and finishes the guard at completion. Calling
//! [`FrozenInput::complete`] returns ownership of the value; dropping the
//! guard discards it. This module does not schedule a thread or stage outputs.

use core::marker::PhantomData;

#[cfg(creusot)]
use creusot_std::prelude::{ensures, requires};

use crate::{channel, Receiver, Sender};

mod sealed {
    use crate::Receiver;

    /// Implemented only by the overflow policies of this module.
    pub trait Sealed {
        /// Returns how many queued events a dispatch drops before freezing one.
        fn excess<T>(receiver: &Receiver<T>, queue_size: usize) -> usize;
    }
}

/// An AADL `Overflow_Handling_Protocol` that [`EventDataInput`] implements.
///
/// The trait is sealed: only [`DropNewest`] and [`DropOldest`] implement it.
pub trait Overflow: sealed::Sealed {}

/// AADL `DropNewest`: a send to a full queue returns the value to the sender,
/// which drops it.
pub struct DropNewest;

/// AADL `DropOldest`: a full queue discards its oldest event for a new one.
///
/// `rtchan` discards at dispatch rather than at send time, which presents the
/// same event as long as the channel never fills.
pub struct DropOldest;

impl sealed::Sealed for DropNewest {
    fn excess<T>(_: &Receiver<T>, _: usize) -> usize {
        0
    }
}

impl sealed::Sealed for DropOldest {
    fn excess<T>(receiver: &Receiver<T>, queue_size: usize) -> usize {
        // When events are queued, this read is the input time: the dispatch
        // freezes one that is already present.
        trim_count(receiver.len(), queue_size)
    }
}

#[cfg_attr(
    creusot,
    ensures(result@ == if count@ > queue_size@ { count@ - queue_size@ } else { 0 })
)]
fn trim_count(count: usize, queue_size: usize) -> usize {
    count.saturating_sub(queue_size)
}

impl Overflow for DropNewest {}
impl Overflow for DropOldest {}

/// OneItem input adapter for one AADL event data input port.
///
/// It owns the receiving end of its channel; use one adapter per input
/// connection.
pub struct EventDataInput<T, P> {
    receiver: Receiver<T>,
    pub(crate) queue_size: usize,
    pub(crate) active: bool,
    policy: PhantomData<P>,
}

/// A dispatch is still active because its frozen guard was forgotten.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct DispatchInProgress;

impl<T> EventDataInput<T, DropNewest> {
    /// Allocates a `DropNewest` connection with AADL `Queue_Size` `queue_size`.
    ///
    /// The channel holds `queue_size` events, so a dispatch never drops one; a
    /// send to a full queue returns its value, and the sender drops it.
    ///
    /// # Panics
    ///
    /// Panics if `queue_size` is zero.
    #[cfg_attr(creusot, requires(queue_size > 0usize))]
    #[cfg_attr(creusot, ensures(result.1.queue_size == queue_size))]
    #[cfg_attr(creusot, ensures(!result.1.active))]
    pub fn new(queue_size: usize) -> (Sender<T>, Self) {
        assert!(queue_size > 0, "queue size must be positive");
        let (sender, receiver) = channel(queue_size);
        (sender, Self::with_receiver(receiver, queue_size))
    }
}

impl<T> EventDataInput<T, DropOldest> {
    /// Allocates a `DropOldest` connection with AADL `Queue_Size` `queue_size`.
    ///
    /// Each dispatch first drops the queued events beyond the newest
    /// `queue_size`, so it freezes the event that AADL `DropOldest` would
    /// present. `max_sends` must bound the sends from the creation of the
    /// connection to the return of the first dispatch, and from the start of
    /// each dispatch to the return of the next. The channel gets capacity
    /// `queue_size + max_sends - 1`, which such sends never fill; a full send
    /// therefore means the bound was violated.
    ///
    /// # Panics
    ///
    /// Panics if `queue_size` or `max_sends` is zero, or if the capacity
    /// overflows `usize`.
    #[cfg_attr(creusot, requires(queue_size > 0usize))]
    #[cfg_attr(creusot, requires(max_sends > 0usize))]
    #[cfg_attr(creusot, requires(queue_size@ + max_sends@ - 1 <= usize::MAX@))]
    #[cfg_attr(creusot, ensures(result.1.queue_size == queue_size))]
    #[cfg_attr(creusot, ensures(!result.1.active))]
    pub fn new(queue_size: usize, max_sends: usize) -> (Sender<T>, Self) {
        assert!(queue_size > 0, "queue size must be positive");
        assert!(max_sends > 0, "send bound must be positive");
        let capacity = queue_size
            .checked_add(max_sends - 1)
            .expect("channel capacity overflows usize");
        let (sender, receiver) = channel(capacity);
        (sender, Self::with_receiver(receiver, queue_size))
    }
}

impl<T, P: Overflow> EventDataInput<T, P> {
    #[cfg_attr(creusot, ensures(result.queue_size == queue_size))]
    #[cfg_attr(creusot, ensures(!result.active))]
    fn with_receiver(receiver: Receiver<T>, queue_size: usize) -> Self {
        Self {
            receiver,
            queue_size,
            active: false,
            policy: PhantomData,
        }
    }

    /// Freezes exactly one input for this dispatch.
    ///
    /// Call this at the AADL input time. The returned guard holds the result
    /// so later arrivals cannot change what this dispatch sees. It borrows
    /// the adapter until completion. A forgotten guard leaves the adapter
    /// active and prevents another receive.
    ///
    /// The input time falls inside this call. Under `DropOldest` with events
    /// queued, it is the length read; otherwise it is the freezing receive.
    /// `DropOldest` drops at most `max_sends - 1` older events in place,
    /// running their destructors.
    pub fn dispatch(&mut self) -> Result<FrozenInput<'_, T, P>, DispatchInProgress> {
        if self.active {
            return Err(DispatchInProgress);
        }
        for _ in 0..P::excess(&self.receiver, self.queue_size) {
            drop(self.receiver.try_recv());
        }
        let value = self.receiver.try_recv();
        self.active = true;
        Ok(FrozenInput {
            adapter: self,
            value,
        })
    }

    /// Returns whether a dispatch now would freeze an event.
    ///
    /// A sporadic scheduler calls this after each wakeup and dispatches while
    /// it returns `true`. Only this adapter removes events, so the answer
    /// stays `true` until the next dispatch. Maps to `len(n)` with `n > 0`.
    pub fn has_pending(&self) -> bool {
        !self.receiver.is_empty()
    }

    /// Returns whether the sender has been dropped.
    pub fn is_abandoned(&self) -> bool {
        self.receiver.is_abandoned()
    }
}

/// Input frozen for one dispatch.
///
/// Call [`complete`](Self::complete) to recover the input value. Dropping the
/// guard also completes the dispatch, but drops the frozen value in place.
#[must_use = "complete() returns the input; dropping this guard discards it"]
pub struct FrozenInput<'a, T, P> {
    adapter: &'a mut EventDataInput<T, P>,
    value: Option<T>,
}

impl<T, P> FrozenInput<'_, T, P> {
    /// Reads the frozen input without removing it.
    ///
    /// Corresponds to AADL `Get_Value`.
    #[cfg_attr(
        creusot,
        ensures(match &self.value {
            Some(value) => result == Some(value),
            None => result == None,
        })
    )]
    pub fn get(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Ends the dispatch and returns ownership of its frozen input.
    ///
    /// The caller can dispose of the value outside the real-time path if its
    /// destructor is not bounded. This method does not allocate or wait.
    pub fn complete(mut self) -> Option<T> {
        self.value.take()
    }
}

impl<T, P> Drop for FrozenInput<'_, T, P> {
    fn drop(&mut self) {
        self.adapter.active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::{DispatchInProgress, DropNewest, DropOldest, EventDataInput};

    #[test]
    fn freezes_one_item_until_completion() {
        let (mut tx, mut input) = EventDataInput::<u8, DropNewest>::new(2);

        tx.try_send(1).unwrap();
        let frozen = input.dispatch().unwrap();
        assert_eq!(frozen.get(), Some(&1));
        tx.try_send(2).unwrap();
        assert_eq!(frozen.get(), Some(&1));
        assert_eq!(frozen.complete(), Some(1));

        let next = input.dispatch().unwrap();
        assert_eq!(next.get(), Some(&2));
        assert_eq!(next.complete(), Some(2));
    }

    #[test]
    fn drop_newest_rejects_send_to_full_queue() {
        let (mut tx, mut input) = EventDataInput::<u8, DropNewest>::new(1);
        assert_eq!(tx.try_send(1), Ok(()));
        assert_eq!(tx.try_send(2), Err(2));
        assert_eq!(input.dispatch().unwrap().complete(), Some(1));
        assert_eq!(input.dispatch().unwrap().complete(), None);
    }

    #[test]
    fn empty_input_stays_empty_until_completion() {
        let (mut tx, mut input) = EventDataInput::<u8, DropNewest>::new(1);

        let frozen = input.dispatch().unwrap();
        assert_eq!(frozen.get(), None);
        tx.try_send(7).unwrap();
        assert_eq!(frozen.get(), None);
        assert_eq!(frozen.complete(), None);

        let next = input.dispatch().unwrap();
        assert_eq!(next.complete(), Some(7));
    }

    #[test]
    fn forgotten_guard_cannot_start_another_dispatch() {
        let (mut tx, mut input) = EventDataInput::<u8, DropNewest>::new(1);

        let frozen = input.dispatch().unwrap();
        core::mem::forget(frozen);
        tx.try_send(7).unwrap();
        assert!(matches!(input.dispatch(), Err(DispatchInProgress)));
    }

    #[test]
    fn dropping_guard_completes_dispatch() {
        let (mut tx, mut input) = EventDataInput::<u8, DropNewest>::new(1);

        let frozen = input.dispatch().unwrap();
        drop(frozen);
        tx.try_send(7).unwrap();
        assert_eq!(input.dispatch().unwrap().complete(), Some(7));
    }

    #[test]
    #[should_panic(expected = "queue size must be positive")]
    fn drop_newest_rejects_zero_queue_size() {
        let _ = EventDataInput::<u8, DropNewest>::new(0);
    }

    #[test]
    fn sporadic_consumer_receives_every_accepted_send() {
        extern crate std;
        use std::thread;
        use std::time::{Duration, Instant};

        // The application wakes the consumer after each accepted send;
        // `unpark` latches like a binary semaphore.
        let (mut tx, mut input) = EventDataInput::<u32, DropNewest>::new(4);
        let consumer = thread::current();
        let producer = thread::spawn(move || {
            for message in 0..1_000 {
                let mut value = message;
                while let Err(back) = tx.try_send(value) {
                    value = back;
                    thread::yield_now();
                }
                consumer.unpark();
            }
        });

        let timeout = Duration::from_secs(5);
        let mut expected = 0;
        while expected < 1_000 {
            let start = Instant::now();
            thread::park_timeout(timeout);
            assert!(
                start.elapsed() < timeout || !input.has_pending(),
                "lost wakeup"
            );
            while input.has_pending() {
                assert_eq!(input.dispatch().unwrap().complete(), Some(expected));
                expected += 1;
            }
        }
        producer.join().unwrap();
    }

    #[test]
    fn drop_oldest_freezes_oldest_of_newest() {
        let (mut tx, mut input) = EventDataInput::<u8, DropOldest>::new(2, 3);

        for message in 1..=4 {
            tx.try_send(message).unwrap();
        }
        assert_eq!(input.dispatch().unwrap().complete(), Some(3));
        assert_eq!(input.dispatch().unwrap().complete(), Some(4));
        assert_eq!(input.dispatch().unwrap().complete(), None);
    }

    #[test]
    #[should_panic(expected = "queue size must be positive")]
    fn drop_oldest_rejects_zero_queue_size() {
        let _ = EventDataInput::<u8, DropOldest>::new(0, 1);
    }

    #[test]
    #[should_panic(expected = "send bound must be positive")]
    fn drop_oldest_rejects_zero_send_bound() {
        let _ = EventDataInput::<u8, DropOldest>::new(1, 0);
    }

    #[test]
    #[should_panic(expected = "channel capacity overflows usize")]
    fn drop_oldest_rejects_overflowing_capacity() {
        let _ = EventDataInput::<u8, DropOldest>::new(usize::MAX, 2);
    }

    #[test]
    fn drop_oldest_matches_aadl_queue_on_bounded_traces() {
        extern crate std;
        use std::collections::VecDeque;
        use std::vec::Vec;

        #[derive(Clone, Copy)]
        enum Op {
            Send(u8),
            Dispatch,
        }

        fn check(queue_size: usize, max_sends: usize, ops: &[Op]) {
            let (mut tx, mut input) = EventDataInput::<u8, DropOldest>::new(queue_size, max_sends);
            let mut aadl = VecDeque::new();
            for op in ops {
                match *op {
                    Op::Send(message) => {
                        if aadl.len() == queue_size {
                            aadl.pop_front();
                        }
                        aadl.push_back(message);
                        assert_eq!(tx.try_send(message), Ok(()));
                    }
                    Op::Dispatch => {
                        let frozen = input.dispatch().unwrap();
                        assert_eq!(frozen.complete(), aadl.pop_front());
                    }
                }
            }
        }

        // Dispatch is atomic in one thread, so `sends` counts the window
        // since the previous dispatch.
        fn explore(n: usize, k: usize, remaining: usize, sends: usize, ops: &mut Vec<Op>) {
            check(n, k, ops);
            if remaining == 0 {
                return;
            }
            if sends < k {
                for message in [0, 1] {
                    ops.push(Op::Send(message));
                    explore(n, k, remaining - 1, sends + 1, ops);
                    ops.pop();
                }
            }
            ops.push(Op::Dispatch);
            explore(n, k, remaining - 1, 0, ops);
            ops.pop();
        }

        for queue_size in [1, 2] {
            for max_sends in [1, 2, 3] {
                explore(queue_size, max_sends, 7, 0, &mut Vec::new());
            }
        }
    }
}
