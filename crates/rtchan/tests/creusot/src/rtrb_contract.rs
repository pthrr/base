//! Trusted functional contract for the `rtrb` calls used by `rtchan`.
//!
//! The two monotone histories record successful sends and receives. Unlike a
//! queue-length snapshot, each history changes only through its own endpoint,
//! so these postconditions remain valid when the peer operates concurrently.
//! Coupling the two endpoint histories requires a shared-state proof and is
//! deliberately not asserted by these endpoint-local contracts.

use creusot_std::{logic::Id, prelude::*};

pub trait ProducerHistory<T> {
    #[logic]
    fn accepted(self) -> Seq<T>;

    /// Receives published by the peer, as seen at this point in time.
    #[logic]
    fn consumed(self) -> Seq<T>;

    #[logic]
    fn capacity(self) -> Int;

    #[logic]
    fn channel_id(self) -> Id;
}

impl<T> ProducerHistory<T> for rtrb::Producer<T> {
    #[logic(opaque)]
    fn accepted(self) -> Seq<T> {
        dead
    }

    #[logic(opaque)]
    fn consumed(self) -> Seq<T> {
        dead
    }

    #[logic(opaque)]
    fn capacity(self) -> Int {
        dead
    }

    #[logic(opaque)]
    fn channel_id(self) -> Id {
        dead
    }
}

pub trait ConsumerHistory<T> {
    /// Published sends observed by this consumer at the operation's
    /// linearization point. A later receive may observe more sends.
    #[logic]
    fn offered(self) -> Seq<T>;

    #[logic]
    fn delivered(self) -> Seq<T>;

    #[logic]
    fn capacity(self) -> Int;

    #[logic]
    fn channel_id(self) -> Id;
}

impl<T> ConsumerHistory<T> for rtrb::Consumer<T> {
    #[logic(opaque)]
    fn offered(self) -> Seq<T> {
        dead
    }

    #[logic(opaque)]
    fn delivered(self) -> Seq<T> {
        dead
    }

    #[logic(opaque)]
    fn capacity(self) -> Int {
        dead
    }

    #[logic(opaque)]
    fn channel_id(self) -> Id {
        dead
    }
}

extern_spec! {
    impl<T> rtrb::RingBuffer<T> {
        #[requires(capacity > 0usize)]
        #[ensures(result.0.accepted() == Seq::empty())]
        #[ensures(result.0.consumed() == Seq::empty())]
        #[ensures(result.1.offered() == Seq::empty())]
        #[ensures(result.1.delivered() == Seq::empty())]
        #[ensures(result.0.capacity() == capacity@)]
        #[ensures(result.1.capacity() == capacity@)]
        #[ensures(result.0.channel_id() == result.1.channel_id())]
        fn new(capacity: usize) -> (rtrb::Producer<T>, rtrb::Consumer<T>);
    }

    impl<T> rtrb::Producer<T> {
        fn is_abandoned(&self) -> bool;

        #[ensures((^self).capacity() == (*self).capacity())]
        #[ensures((^self).channel_id() == (*self).channel_id())]
        #[ensures((^self).consumed().len() <= (^self).accepted().len())]
        #[ensures((^self).consumed()
            == (^self).accepted().subsequence(0, (^self).consumed().len()))]
        #[ensures((^self).accepted().len() - (^self).consumed().len() <= (^self).capacity())]
        #[ensures((*self).consumed().len() <= (^self).consumed().len())]
        #[ensures((*self).consumed()
            == (^self).consumed().subsequence(0, (*self).consumed().len()))]
        #[ensures(match result {
            Ok(()) => (^self).accepted() == (*self).accepted().push_back(value),
            Err(rtrb::PushError::Full(back)) =>
                back == value && (^self).accepted() == (*self).accepted()
                    && (*self).accepted().len() - (^self).consumed().len()
                        == (*self).capacity(),
        })]
        #[ensures((*self).accepted().len() - (*self).consumed().len() < (*self).capacity()
            ==> match result { Ok(()) => true, Err(_) => false })]
        fn push(&mut self, value: T) -> Result<(), rtrb::PushError<T>>;
    }

    impl<T> rtrb::Consumer<T> {
        fn is_abandoned(&self) -> bool;

        #[ensures((^self).capacity() == (*self).capacity())]
        #[ensures((^self).channel_id() == (*self).channel_id())]
        #[ensures((^self).delivered().len() <= (^self).offered().len())]
        #[ensures((^self).offered().len() - (^self).delivered().len() <= (^self).capacity())]
        #[ensures((*self).offered().len() <= (^self).offered().len())]
        #[ensures((*self).offered()
            == (^self).offered().subsequence(0, (*self).offered().len()))]
        #[ensures(forall<index: Int> 0 <= index && index < (*self).offered().len()
            ==> (*self).offered()[index] == (^self).offered()[index])]
        #[ensures(match result {
            Ok(value) => (^self).delivered() == (*self).delivered().push_back(value)
                && (*self).delivered().len() < (^self).offered().len()
                && value == (^self).offered()[(*self).delivered().len()],
            Err(rtrb::PopError::Empty) => (^self).delivered() == (*self).delivered()
                && (^self).offered().len() == (^self).delivered().len(),
        })]
        #[ensures((*self).offered().len() > (*self).delivered().len()
            ==> match result { Ok(_) => true, Err(_) => false })]
        fn pop(&mut self) -> Result<T, rtrb::PopError>;

        // A shared-reference specification cannot update the ghost history
        // when a concurrent producer publishes. The mutable observation
        // wrapper in channel.rs supplies that boundary contract.
        #[ensures(result@ <= self.capacity())]
        fn slots(&self) -> usize;
    }
}
