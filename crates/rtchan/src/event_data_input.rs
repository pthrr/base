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
use creusot_std::prelude::{
    ensures, invariant, logic, requires, snapshot, Int, Invariant, Seq, Snapshot,
};

use crate::{channel, Receiver, Sender};

/// The AADL pending queue at input time, derived from the observed FIFO.
#[cfg(creusot)]
#[logic(open, inline)]
fn aadl_pending<T>(observed: Seq<T>, delivered: Int, queue_size: Int, drop_oldest: bool) -> Seq<T> {
    let count = observed.len() - delivered;
    let discarded = if drop_oldest && count > queue_size {
        count - queue_size
    } else {
        0
    };
    observed.subsequence(delivered + discarded, observed.len())
}

mod sealed {
    #[cfg(creusot)]
    use creusot_std::prelude::ensures;

    /// Implemented only by the overflow policies of this module.
    pub trait Sealed {
        const DROP_OLDEST: bool;

        /// Returns how many observed events a dispatch drops before freezing one.
        #[cfg_attr(
            creusot,
            ensures(result@ == if Self::DROP_OLDEST && count@ > queue_size@ {
                count@ - queue_size@
            } else { 0 })
        )]
        fn excess(count: usize, queue_size: usize) -> usize;
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
    const DROP_OLDEST: bool = false;

    #[cfg_attr(creusot, ensures(result == 0usize))]
    fn excess(_count: usize, _: usize) -> usize {
        0
    }
}

impl sealed::Sealed for DropOldest {
    const DROP_OLDEST: bool = true;

    #[cfg_attr(
        creusot,
        ensures(result@ == if count@ > queue_size@ {
            count@ - queue_size@
        } else { 0 })
    )]
    fn excess(count: usize, queue_size: usize) -> usize {
        trim_count(count, queue_size)
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

#[cfg(creusot)]
impl<T, P> Invariant for EventDataInput<T, P> {
    #[logic]
    fn invariant(self) -> bool {
        pearlite! {
            self.queue_size > 0usize && self.receiver.capacity() >= self.queue_size@
        }
    }
}

/// A dispatch is still active because its frozen guard was forgotten.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct DispatchInProgress;

/// Proof-only record of the FIFO observations made by an internal dispatch.
pub(crate) struct DispatchObservation<T> {
    #[cfg(creusot)]
    observed: Snapshot<Seq<T>>,
    #[cfg(creusot)]
    received: Snapshot<Seq<T>>,
    #[cfg(creusot)]
    start: Snapshot<Int>,
    #[cfg(creusot)]
    selected: Snapshot<Int>,
    #[cfg(creusot)]
    end: Snapshot<Int>,
    #[cfg(not(creusot))]
    marker: PhantomData<T>,
}

impl<T> DispatchObservation<T> {
    #[cfg(creusot)]
    #[logic]
    pub fn observed(self) -> Seq<T> {
        *self.observed
    }

    #[cfg(creusot)]
    #[logic]
    pub fn received(self) -> Seq<T> {
        *self.received
    }

    #[cfg(creusot)]
    #[logic]
    pub fn start_index(self) -> Int {
        *self.start
    }

    #[cfg(creusot)]
    #[logic]
    pub fn selected_index(self) -> Int {
        *self.selected
    }

    #[cfg(creusot)]
    #[logic]
    pub fn end_index(self) -> Int {
        *self.end
    }
}

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
    #[cfg_attr(creusot, ensures(result.0.capacity() == queue_size@))]
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
    #[cfg_attr(creusot, ensures(result.0.capacity() == queue_size@ + max_sends@ - 1))]
    #[cfg_attr(
        creusot,
        ensures(forall<queued: Int> 0 <= queued && queued < queue_size@
            ==> queued + max_sends@ <= result.0.capacity())
    )]
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
    #[cfg(creusot)]
    #[logic]
    pub fn pending(self) -> Int {
        self.receiver.pending()
    }

    #[cfg_attr(creusot, requires(queue_size > 0usize))]
    #[cfg_attr(creusot, requires(receiver.capacity() >= queue_size@))]
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
    /// Align its length read with the AADL input time. The returned guard
    /// holds the result so later arrivals cannot change what this dispatch sees. It borrows
    /// the adapter until completion. A forgotten guard leaves the adapter
    /// active and prevents another receive.
    ///
    /// The input time is the length read inside this call for either policy.
    /// `DropOldest` drops at most `max_sends - 1` older events in place,
    /// running their destructors.
    #[cfg_attr(
        creusot,
        ensures(match result {
            Ok(guard) => !(*self).active
                && guard.frozen() == aadl_pending(
                    guard.observed(), guard.start_index(), (*self).queue_size@,
                    P::DROP_OLDEST).get(0)
                && (forall<index: Int> 0 <= index && index < guard.observed().len()
                    ==> guard.received()[index] == guard.observed()[index])
                && (match guard.frozen() { Some(_) => true, None => false })
                    == (guard.observed().len() > guard.start_index())
                && guard.end_index() == if guard.observed().len() > guard.start_index() {
                    guard.selected_index() + 1
                } else { guard.start_index() }
                && guard.selected_index() == guard.start_index()
                    + if P::DROP_OLDEST
                        && guard.observed().len() - guard.start_index() > (*self).queue_size@ {
                        guard.observed().len() - guard.start_index() - (*self).queue_size@
                    } else { 0 }
                && (guard.observed().len() > guard.start_index() ==>
                    guard.selected_index() < guard.observed().len()
                    && guard.frozen() == Some(guard.observed()[guard.selected_index()]))
                && match guard.frozen() {
                    Some(value) => guard.selected_index() < guard.received().len()
                        && value == guard.received()[guard.selected_index()],
                    None => guard.observed().len() == guard.selected_index(),
                },
            Err(_) => (*self).active,
        })
    )]
    #[cfg_attr(
        creusot,
        ensures(match result {
            Ok(guard) => guard.queue_size() == (*self).queue_size,
            Err(_) => (^self).queue_size == (*self).queue_size,
        })
    )]
    pub fn dispatch(&mut self) -> Result<FrozenInput<'_, T, P>, DispatchInProgress> {
        let (value, _observation) = self.freeze()?;
        Ok(FrozenInput {
            adapter: self,
            value,
            #[cfg(creusot)]
            observed: _observation.observed,
            #[cfg(creusot)]
            received: _observation.received,
            #[cfg(creusot)]
            start: _observation.start,
            #[cfg(creusot)]
            selected: _observation.selected,
            #[cfg(creusot)]
            end: _observation.end,
        })
    }

    #[cfg_attr(creusot, ensures((^self).queue_size == (*self).queue_size))]
    #[cfg_attr(creusot, ensures(match result {
        Ok((value, observation)) => !(*self).active
            && (^self).active
            && value == aadl_pending(
                observation.observed(), observation.start_index(), (*self).queue_size@,
                P::DROP_OLDEST).get(0)
            && (forall<index: Int> 0 <= index && index < observation.observed().len()
                ==> observation.received()[index] == observation.observed()[index])
            && (^self).receiver.observed_queue()
                == observation.observed().subsequence(
                    observation.end_index(), observation.observed().len())
                    .concat(observation.received().subsequence(
                        observation.observed().len(), observation.received().len()))
            && (match value { Some(_) => true, None => false })
                == (observation.observed().len() > observation.start_index())
            && observation.end_index() == if observation.observed().len() > observation.start_index() {
                observation.selected_index() + 1
            } else { observation.start_index() }
            && observation.selected_index() == observation.start_index()
                + if P::DROP_OLDEST
                    && observation.observed().len() - observation.start_index() > (*self).queue_size@ {
                    observation.observed().len() - observation.start_index() - (*self).queue_size@
                } else { 0 }
            && (observation.observed().len() > observation.start_index() ==>
                observation.selected_index() < observation.observed().len()
                && value == Some(observation.observed()[observation.selected_index()]))
            && match value {
                Some(value) => observation.selected_index() < observation.received().len()
                    && value == observation.received()[observation.selected_index()],
                None => observation.observed().len() == observation.selected_index(),
            },
        Err(_) => (*self).active,
    }))]
    fn freeze(&mut self) -> Result<(Option<T>, DispatchObservation<T>), DispatchInProgress> {
        if self.active {
            return Err(DispatchInProgress);
        }
        let count = self.receiver.len();
        let mut excess = P::excess(count, self.queue_size);
        #[cfg(creusot)]
        let observed = snapshot!(self.receiver.offered());
        #[cfg(creusot)]
        let delivered_before = snapshot!(self.receiver.delivered().len());
        #[cfg(creusot)]
        let original_excess = snapshot!(excess@);
        #[cfg_attr(
            creusot,
            invariant(self.receiver.capacity() >= self.queue_size@)
        )]
        #[cfg_attr(creusot, invariant(excess@ == 0 || self.receiver.pending() >= excess@))]
        #[cfg_attr(
            creusot,
            invariant(self.receiver.delivered().len()
                == *delivered_before + *original_excess - excess@)
        )]
        #[cfg_attr(
            creusot,
            invariant(self.receiver.offered().len() >= observed.len())
        )]
        #[cfg_attr(
            creusot,
            invariant(forall<index: Int> 0 <= index && index < observed.len()
                ==> self.receiver.offered()[index] == observed[index])
        )]
        #[cfg_attr(
            creusot,
            invariant(*delivered_before + *original_excess < observed.len() ==>
                self.receiver.offered()[*delivered_before + *original_excess]
                    == observed[*delivered_before + *original_excess])
        )]
        while excess > 0 {
            let dropped = self.receiver.try_recv();
            #[cfg(creusot)]
            creusot_std::prelude::proof_assert!(match dropped {
                Some(_) => true,
                None => false,
            });
            drop(dropped);
            excess -= 1;
        }
        let value = if count == 0 {
            None
        } else {
            self.receiver.try_recv()
        };
        #[cfg(creusot)]
        let received = snapshot!(self.receiver.offered());
        #[cfg(creusot)]
        creusot_std::prelude::proof_assert!(
            *delivered_before + *original_excess < observed.len()
                ==> value == Some(observed[*delivered_before + *original_excess])
        );
        self.active = true;
        Ok((
            value,
            DispatchObservation {
                #[cfg(creusot)]
                observed,
                #[cfg(creusot)]
                received,
                #[cfg(creusot)]
                start: delivered_before,
                #[cfg(creusot)]
                selected: snapshot!(*delivered_before + *original_excess),
                #[cfg(creusot)]
                end: snapshot!(self.receiver.delivered().len()),
                #[cfg(not(creusot))]
                marker: PhantomData,
            },
        ))
    }

    /// Completes an internal dispatch before returning its value to the data port.
    #[cfg_attr(creusot, requires(!self.active))]
    #[cfg_attr(creusot, ensures(!(^self).active))]
    #[cfg_attr(creusot, ensures((^self).queue_size == (*self).queue_size))]
    #[cfg_attr(
        creusot,
        ensures(forall<index: Int> 0 <= index && index < result.1.observed().len()
            ==> result.1.received()[index] == result.1.observed()[index])
    )]
    #[cfg_attr(
        creusot,
        ensures(result.1.selected_index() == result.1.start_index()
            + if P::DROP_OLDEST
                && result.1.observed().len() - result.1.start_index() > (*self).queue_size@ {
                result.1.observed().len() - result.1.start_index() - (*self).queue_size@
            } else { 0 })
    )]
    #[cfg_attr(creusot, ensures(result.0 == aadl_pending(
        result.1.observed(), result.1.start_index(), (*self).queue_size@,
        P::DROP_OLDEST).get(0)))]
    #[cfg_attr(
        creusot,
        ensures((match result.0 { Some(_) => true, None => false })
            == (result.1.observed().len() > result.1.start_index()))
    )]
    #[cfg_attr(
        creusot,
        ensures(result.1.end_index() == if result.1.observed().len() > result.1.start_index() {
            result.1.selected_index() + 1
        } else { result.1.start_index() })
    )]
    #[cfg_attr(
        creusot,
        ensures(result.1.observed().len() > result.1.start_index() ==>
            result.1.selected_index() < result.1.observed().len()
                && result.0 == Some(result.1.observed()[result.1.selected_index()]))
    )]
    #[cfg_attr(
        creusot,
        ensures(match result.0 {
            Some(value) => result.1.selected_index() < result.1.received().len()
                && value == result.1.received()[result.1.selected_index()],
            None => result.1.observed().len() == result.1.selected_index(),
        })
    )]
    pub(crate) fn dispatch_value(&mut self) -> (Option<T>, DispatchObservation<T>) {
        let (value, observation) = match self.freeze() {
            Ok((value, observation)) => (value, observation),
            Err(_) => {
                let observation = DispatchObservation {
                    #[cfg(creusot)]
                    observed: snapshot!(Seq::empty()),
                    #[cfg(creusot)]
                    received: snapshot!(Seq::empty()),
                    #[cfg(creusot)]
                    start: snapshot!(0),
                    #[cfg(creusot)]
                    selected: snapshot!(0),
                    #[cfg(creusot)]
                    end: snapshot!(0),
                    #[cfg(not(creusot))]
                    marker: PhantomData,
                };
                (None, observation)
            }
        };
        self.active = false;
        (value, observation)
    }

    /// Returns whether a dispatch now would freeze an event.
    ///
    /// A sporadic scheduler calls this after each wakeup and dispatches while
    /// it returns `true`. Only this adapter removes events, so the answer
    /// stays `true` until the next dispatch. Maps to `len(n)` with `n > 0`.
    #[cfg_attr(creusot, ensures(result == ((^self).pending() > 0)))]
    pub fn has_pending(&mut self) -> bool {
        self.receiver.len() > 0
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
    #[cfg(creusot)]
    observed: Snapshot<Seq<T>>,
    #[cfg(creusot)]
    received: Snapshot<Seq<T>>,
    #[cfg(creusot)]
    start: Snapshot<Int>,
    #[cfg(creusot)]
    selected: Snapshot<Int>,
    #[cfg(creusot)]
    end: Snapshot<Int>,
}

impl<T, P> FrozenInput<'_, T, P> {
    #[cfg(creusot)]
    #[logic]
    pub fn queue_size(self) -> usize {
        self.adapter.queue_size
    }

    #[cfg(creusot)]
    #[logic]
    pub fn observed(self) -> Seq<T> {
        *self.observed
    }

    #[cfg(creusot)]
    #[logic]
    pub fn received(self) -> Seq<T> {
        *self.received
    }

    #[cfg(creusot)]
    #[logic]
    pub fn selected_index(self) -> Int {
        *self.selected
    }

    #[cfg(creusot)]
    #[logic]
    pub fn end_index(self) -> Int {
        *self.end
    }

    #[cfg(creusot)]
    #[logic]
    pub fn start_index(self) -> Int {
        *self.start
    }

    #[cfg(creusot)]
    #[logic]
    pub fn frozen(self) -> Option<T> {
        self.value
    }

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
    #[cfg_attr(creusot, ensures(result == self.frozen()))]
    #[cfg_attr(
        creusot,
        ensures((^self.adapter).queue_size == (*self.adapter).queue_size)
    )]
    pub fn complete(mut self) -> Option<T> {
        self.value.take()
    }
}

impl<T, P> Drop for FrozenInput<'_, T, P> {
    #[cfg_attr(creusot, ensures(!(^self).adapter.active))]
    #[cfg_attr(
        creusot,
        ensures((^self).adapter.queue_size == (*self).adapter.queue_size)
    )]
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
