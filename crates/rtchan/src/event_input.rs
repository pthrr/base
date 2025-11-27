//! Input adapter for an AADL event port. An occurrence carries no data.

use crate::{
    DispatchInProgress, DropNewest, DropOldest, EventDataInput, FrozenInput, Overflow, Sender,
};

#[cfg(creusot)]
use creusot_std::prelude::requires;

/// One AADL event input port, implemented with a unit-valued event data port.
pub struct EventInput<P> {
    inner: EventDataInput<(), P>,
}

impl EventInput<DropNewest> {
    /// Allocates an event queue with `queue_size` slots.
    #[cfg_attr(creusot, requires(queue_size > 0usize))]
    pub fn new(queue_size: usize) -> (Sender<()>, Self) {
        let (sender, inner) = EventDataInput::<(), DropNewest>::new(queue_size);
        (sender, Self { inner })
    }
}

impl EventInput<DropOldest> {
    /// Allocates an event queue with `queue_size` visible slots and a send bound.
    #[cfg_attr(creusot, requires(queue_size > 0usize))]
    #[cfg_attr(creusot, requires(max_sends > 0usize))]
    #[cfg_attr(creusot, requires(queue_size@ + max_sends@ - 1 <= usize::MAX@))]
    pub fn new(queue_size: usize, max_sends: usize) -> (Sender<()>, Self) {
        let (sender, inner) = EventDataInput::<(), DropOldest>::new(queue_size, max_sends);
        (sender, Self { inner })
    }
}

impl<P: Overflow> EventInput<P> {
    /// Freezes one occurrence for the current dispatch.
    pub fn dispatch(&mut self) -> Result<FrozenInput<'_, (), P>, DispatchInProgress> {
        self.inner.dispatch()
    }

    /// Reports whether a dispatch would freeze an occurrence.
    pub fn has_pending(&self) -> bool {
        self.inner.has_pending()
    }

    /// Reports whether the producer was dropped.
    pub fn is_abandoned(&self) -> bool {
        self.inner.is_abandoned()
    }
}

#[cfg(test)]
mod tests {
    use super::EventInput;
    use crate::{DropNewest, DropOldest};

    #[test]
    fn queues_occurrences_without_payload() {
        let (mut sender, mut input) = EventInput::<DropNewest>::new(2);
        assert_eq!(sender.try_send(()), Ok(()));
        assert_eq!(sender.try_send(()), Ok(()));
        assert_eq!(sender.try_send(()), Err(()));
        assert_eq!(input.dispatch().unwrap().complete(), Some(()));
        assert_eq!(input.dispatch().unwrap().complete(), Some(()));
        assert_eq!(input.dispatch().unwrap().complete(), None);

        let (mut sender, mut input) = EventInput::<DropOldest>::new(1, 3);
        for _ in 0..3 {
            assert_eq!(sender.try_send(()), Ok(()));
        }
        assert!(input.has_pending());
        assert_eq!(input.dispatch().unwrap().complete(), Some(()));
        assert!(!input.has_pending());
    }
}
