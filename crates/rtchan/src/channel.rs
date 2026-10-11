//! Bounded SPSC channel for Linux threads and embedded tasks.
//!
//! [`channel`] allocates fixed-capacity storage during construction. Afterwards,
//! [`Sender::try_send`] and [`Receiver::try_recv`] are wait-free and do not
//! allocate. The final endpoint drop frees the buffer; construct and destroy
//! channels outside real-time code.
//! [`crate::EventDataInput`] wraps a receiver to freeze one event per dispatch.
//!
//! ```
//! use rtchan::channel;
//!
//! let (mut tx, mut rx) = channel::<u8>(2);
//! assert_eq!(tx.try_send(7), Ok(()));
//! assert_eq!(rx.try_recv(), Some(7));
//! ```

#[cfg(creusot)]
use crate::rtrb_contract::{ConsumerHistory, ProducerHistory};
#[cfg(creusot)]
use creusot_std::prelude::{ensures, logic, requires, trusted, Int, Invariant, Seq};

/// Boundary observation: `rtrb::Consumer::slots` loads the producer's tail.
/// Its ghost offered history may advance even though `rtrb` takes `&self`.
#[cfg_attr(creusot, trusted)]
#[cfg_attr(creusot, ensures((^consumer).capacity() == (*consumer).capacity()))]
#[cfg_attr(creusot, ensures((^consumer).channel_id() == (*consumer).channel_id()))]
#[cfg_attr(creusot, ensures((^consumer).delivered() == (*consumer).delivered()))]
#[cfg_attr(creusot, ensures((*consumer).offered().len() <= (^consumer).offered().len()))]
#[cfg_attr(creusot, ensures((*consumer).offered()
    == (^consumer).offered().subsequence(0, (*consumer).offered().len())))]
#[cfg_attr(creusot, ensures((^consumer).offered().len() - (^consumer).delivered().len()
    <= (^consumer).capacity()))]
#[cfg_attr(creusot, ensures(result@ == (^consumer).offered().len()
    - (^consumer).delivered().len()))]
fn observe_slots<T>(consumer: &mut Receiver<T>) -> usize {
    consumer.rx.slots()
}

#[cfg_attr(
    creusot,
    ensures(match input {
        Ok(()) => result == Ok(()),
        Err(rtrb::PushError::Full(message)) => result == Err(message),
    })
)]
fn send_outcome<T>(input: Result<(), rtrb::PushError<T>>) -> Result<(), T> {
    match input {
        Ok(()) => Ok(()),
        Err(rtrb::PushError::Full(message)) => Err(message),
    }
}

#[cfg_attr(
    creusot,
    ensures(match input {
        Ok(message) => result == Some(message),
        Err(rtrb::PopError::Empty) => result == None,
    })
)]
fn recv_outcome<T>(input: Result<T, rtrb::PopError>) -> Option<T> {
    match input {
        Ok(message) => Some(message),
        Err(rtrb::PopError::Empty) => None,
    }
}

/// The sending endpoint of one SPSC channel.
pub struct Sender<T> {
    tx: rtrb::Producer<T>,
}

/// The receiving endpoint of one SPSC channel.
pub struct Receiver<T> {
    rx: rtrb::Consumer<T>,
}

#[cfg(creusot)]
impl<T> Invariant for Sender<T> {
    #[logic]
    fn invariant(self) -> bool {
        pearlite! {
            self.consumed().len() <= self.accepted().len()
                && self.consumed()
                    == self.accepted().subsequence(0, self.consumed().len())
                && self.accepted().len() - self.consumed().len() <= self.capacity()
        }
    }
}

#[cfg(creusot)]
impl<T> Invariant for Receiver<T> {
    #[logic]
    fn invariant(self) -> bool {
        pearlite! {
            self.delivered().len() <= self.offered().len()
                && self.offered().len() - self.delivered().len() <= self.capacity()
                && self.delivered()
                    == self.offered().subsequence(0, self.delivered().len())
        }
    }
}

/// Allocates a bounded channel and returns its unique sender and receiver.
///
/// `cap` must be positive and corresponds to `RTCHAN_CHANNEL` in
/// `models/channel.m4`.
/// An embedded application must supply a global allocator for construction
/// and final drop.
///
/// # Panics
///
/// Panics if `cap` is zero. Allocation failure follows the application's
/// allocator policy.
#[cfg_attr(creusot, requires(cap > 0usize))]
#[cfg_attr(creusot, ensures(result.0.accepted() == Seq::empty()))]
#[cfg_attr(creusot, ensures(result.0.consumed() == Seq::empty()))]
#[cfg_attr(creusot, ensures(result.1.offered() == Seq::empty()))]
#[cfg_attr(creusot, ensures(result.1.delivered() == Seq::empty()))]
#[cfg_attr(creusot, ensures(result.0.accepted() == result.1.offered()))]
#[cfg_attr(creusot, ensures(result.0.consumed() == result.1.delivered()))]
#[cfg_attr(creusot, ensures(result.0.channel_id() == result.1.channel_id()))]
#[cfg_attr(creusot, ensures(result.0.capacity() == cap@))]
#[cfg_attr(creusot, ensures(result.1.capacity() == cap@))]
pub fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    assert!(cap > 0, "channel capacity must be positive");
    let (tx, rx) = rtrb::RingBuffer::new(cap);
    (Sender { tx }, Receiver { rx })
}

impl<T> Sender<T> {
    #[cfg(creusot)]
    #[logic]
    pub fn accepted(self) -> Seq<T> {
        self.tx.accepted()
    }

    #[cfg(creusot)]
    #[logic]
    pub fn consumed(self) -> Seq<T> {
        self.tx.consumed()
    }

    #[cfg(creusot)]
    #[logic]
    pub fn channel_id(self) -> creusot_std::logic::Id {
        self.tx.channel_id()
    }

    #[cfg(creusot)]
    #[logic]
    pub fn capacity(self) -> Int {
        self.tx.capacity()
    }

    /// Attempts one send without waiting or allocating.
    ///
    /// `Ok(())` maps to `send_ok(m)`; `Err(m)` maps to `send_full(m)` while both
    /// endpoints are alive. The returned value remains owned by the caller.
    #[cfg_attr(
        creusot,
        ensures(match result {
            Ok(()) => (^self).accepted() == (*self).accepted().push_back(message),
            Err(back) => back == message && (^self).accepted() == (*self).accepted(),
        })
    )]
    #[cfg_attr(creusot, ensures((^self).channel_id() == (*self).channel_id()))]
    #[cfg_attr(creusot, ensures((^self).capacity() == (*self).capacity()))]
    #[cfg_attr(creusot, ensures((^self).consumed()
        == (^self).accepted().subsequence(0, (^self).consumed().len())))]
    pub fn try_send(&mut self, message: T) -> Result<(), T> {
        if self.tx.is_abandoned() {
            return Err(message);
        }
        send_outcome(self.tx.push(message))
    }

    /// Returns whether the receiver has been dropped.
    pub fn is_abandoned(&self) -> bool {
        self.tx.is_abandoned()
    }
}

impl<T> Receiver<T> {
    #[cfg(creusot)]
    #[logic]
    pub fn delivered(self) -> Seq<T> {
        self.rx.delivered()
    }

    #[cfg(creusot)]
    #[logic]
    pub fn offered(self) -> Seq<T> {
        self.rx.offered()
    }

    #[cfg(creusot)]
    #[logic(open)]
    pub fn pending(self) -> creusot_std::prelude::Int {
        self.offered().len() - self.delivered().len()
    }

    /// FIFO values visible at this receiver's latest observation.
    #[cfg(creusot)]
    #[logic(open, inline)]
    pub fn observed_queue(self) -> Seq<T> {
        self.offered()
            .subsequence(self.delivered().len(), self.offered().len())
    }

    #[cfg(creusot)]
    #[logic]
    pub fn channel_id(self) -> creusot_std::logic::Id {
        self.rx.channel_id()
    }

    #[cfg(creusot)]
    #[logic]
    pub fn capacity(self) -> Int {
        self.rx.capacity()
    }

    /// Attempts one receive without waiting or allocating.
    ///
    /// `Some(m)` maps to `recv_ok(m)` and `None` maps to `recv_empty` while
    /// both endpoints are alive. Queued messages remain readable after the
    /// sender is dropped.
    #[cfg_attr(
        creusot,
        ensures(match result {
            Some(message) => (^self).delivered() == (*self).delivered().push_back(message)
                && (*self).delivered().len() < (^self).offered().len()
                && message == (^self).offered()[(*self).delivered().len()],
            None => (^self).delivered() == (*self).delivered()
                && (^self).offered().len() == (^self).delivered().len(),
        })
    )]
    #[cfg_attr(
        creusot,
        ensures((*self).offered().len() > (*self).delivered().len()
            ==> match result { Some(_) => true, None => false })
    )]
    #[cfg_attr(
        creusot,
        ensures((*self).offered().len() > (*self).delivered().len()
            ==> match result {
                Some(message) => message == (*self).offered()[(*self).delivered().len()],
                None => true,
            })
    )]
    #[cfg_attr(creusot, ensures((^self).channel_id() == (*self).channel_id()))]
    #[cfg_attr(creusot, ensures((^self).capacity() == (*self).capacity()))]
    #[cfg_attr(creusot, ensures((^self).offered().len() >= (*self).offered().len()))]
    #[cfg_attr(
        creusot,
        ensures(forall<index: Int> 0 <= index && index < (*self).offered().len()
            ==> (*self).offered()[index] == (^self).offered()[index])
    )]
    pub fn try_recv(&mut self) -> Option<T> {
        recv_outcome(self.rx.pop())
    }

    /// Returns whether the sender has been dropped.
    pub fn is_abandoned(&self) -> bool {
        self.rx.is_abandoned()
    }

    /// Returns the number of queued messages at the atomic tail load.
    ///
    /// Maps to `len(n)`. Only the receiver removes messages, so concurrent
    /// sends can only increase the count.
    #[cfg_attr(
        creusot,
        ensures(result@ == (^self).offered().len() - (^self).delivered().len())
    )]
    #[cfg_attr(creusot, ensures(result@ == (^self).observed_queue().len()))]
    #[cfg_attr(creusot, ensures((^self).capacity() == (*self).capacity()))]
    #[cfg_attr(creusot, ensures((^self).delivered() == (*self).delivered()))]
    #[cfg_attr(creusot, ensures((*self).offered().len() <= (^self).offered().len()))]
    #[cfg_attr(creusot, ensures((*self).offered()
        == (^self).offered().subsequence(0, (*self).offered().len())))]
    pub fn len(&mut self) -> usize {
        observe_slots(self)
    }

    /// Returns whether no message was queued at the time of the call.
    #[cfg_attr(
        creusot,
        ensures(result == ((^self).offered().len() == (^self).delivered().len()))
    )]
    #[cfg_attr(creusot, ensures((^self).capacity() == (*self).capacity()))]
    #[cfg_attr(creusot, ensures((^self).delivered() == (*self).delivered()))]
    #[cfg_attr(creusot, ensures((*self).offered().len() <= (^self).offered().len()))]
    #[cfg_attr(creusot, ensures((*self).offered()
        == (^self).offered().subsequence(0, (*self).offered().len())))]
    pub fn is_empty(&mut self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::channel;

    #[test]
    #[should_panic(expected = "channel capacity must be positive")]
    fn zero_capacity_panics() {
        let _ = channel::<u8>(0);
    }

    #[test]
    fn fifo_full_and_drop_newest() {
        let (mut tx, mut rx) = channel::<u8>(2);
        assert_eq!(rx.try_recv(), None);
        assert_eq!(tx.try_send(1), Ok(()));
        assert_eq!(tx.try_send(2), Ok(()));
        assert_eq!(tx.try_send(3), Err(3));
        assert_eq!(rx.try_recv(), Some(1));
        assert_eq!(rx.try_recv(), Some(2));
    }

    #[test]
    fn separate_length_observations_see_new_sends() {
        let (mut sender, mut receiver) = channel::<u8>(2);
        assert_eq!(receiver.len(), 0);
        sender.try_send(7).unwrap();
        assert_eq!(receiver.len(), 1);
        assert!(!receiver.is_empty());
    }

    #[test]
    fn all_short_traces_match_bounded_fifo() {
        use std::collections::VecDeque;
        use std::vec::Vec;

        #[derive(Clone, Copy)]
        enum Op {
            Send(u8),
            Recv,
        }

        fn check(cap: usize, ops: &[Op]) {
            let (mut tx, mut rx) = channel::<u8>(cap);
            let mut queue = VecDeque::new();
            for op in ops {
                match *op {
                    Op::Send(message) => {
                        let expected = if queue.len() < cap {
                            queue.push_back(message);
                            Ok(())
                        } else {
                            Err(message)
                        };
                        assert_eq!(tx.try_send(message), expected);
                    }
                    Op::Recv => assert_eq!(rx.try_recv(), queue.pop_front()),
                }
            }
        }

        fn explore(cap: usize, remaining: usize, ops: &mut Vec<Op>) {
            check(cap, ops);
            if remaining == 0 {
                return;
            }
            for op in [Op::Send(0), Op::Send(1), Op::Recv] {
                ops.push(op);
                explore(cap, remaining - 1, ops);
                ops.pop();
            }
        }

        for cap in [1, 2] {
            explore(cap, 6, &mut Vec::new());
        }
    }

    #[test]
    fn abandonment_keeps_queued_messages_readable() {
        let (mut tx, mut rx) = channel::<u8>(1);
        tx.try_send(9).unwrap();
        drop(tx);
        assert!(rx.is_abandoned());
        assert_eq!(rx.try_recv(), Some(9));
        assert_eq!(rx.try_recv(), None);
    }

    #[test]
    fn send_after_receiver_drop_returns_value() {
        let (mut tx, rx) = channel::<u8>(1);
        drop(rx);
        assert!(tx.is_abandoned());
        assert_eq!(tx.try_send(3), Err(3));
    }

    #[test]
    fn threads_preserve_fifo() {
        let (mut tx, mut rx) = channel::<u32>(8);
        let producer = std::thread::spawn(move || {
            for expected in 0..1_000 {
                let mut value = expected;
                loop {
                    match tx.try_send(value) {
                        Ok(()) => break,
                        Err(back) => {
                            assert!(!tx.is_abandoned(), "receiver dropped early");
                            value = back;
                            std::thread::yield_now();
                        }
                    }
                }
            }
        });
        for expected in 0..1_000 {
            loop {
                if let Some(value) = rx.try_recv() {
                    assert_eq!(value, expected);
                    break;
                }
                assert!(!rx.is_abandoned(), "sender dropped early");
                std::thread::yield_now();
            }
        }
        producer.join().unwrap();
    }
}
