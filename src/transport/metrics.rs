use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use crate::transport::stream_ext::StatMonitor;

#[derive(Debug, Clone)]
pub struct TransportMetrics {
    received: Arc<AtomicUsize>,
    transmitted: Arc<AtomicUsize>,
    relay_counter: Counter,
    client_counter: Counter,
    remote_counter: Counter,
}

#[derive(Debug, Clone)]
pub struct Counter {
    current: Arc<AtomicUsize>,
    accumulated: Arc<AtomicUsize>,
}

impl Counter {
    #[inline]
    pub fn new(n: usize) -> Counter {
        let current = Arc::new(AtomicUsize::new(n));
        let accumulated = Arc::new(AtomicUsize::new(n));
        Counter { current, accumulated }
    }

    #[inline]
    pub fn zero() -> Counter { Counter::new(0) }

    #[inline]
    pub fn increase(&self) -> usize {
        let prev = self.current.fetch_add(1, Ordering::SeqCst);
        self.accumulated.fetch_add(1, Ordering::SeqCst);
        prev
    }

    #[inline]
    pub fn decrease(&self) -> usize {
        let prev = self.current.fetch_sub(1, Ordering::SeqCst);
        prev
    }

    #[inline]
    pub fn current(&self) -> usize { self.current.load(Ordering::Acquire) }

    pub fn accumulated(&self) -> usize { self.accumulated.load(Ordering::Acquire) }
}

pub struct CounterHelper(Counter);

impl CounterHelper {
    #[inline]
    pub fn count(counter: Counter) -> (CounterHelper, usize) {
        let prev = counter.increase();
        (CounterHelper(counter), prev)
    }
}

impl Drop for CounterHelper {
    fn drop(&mut self) { self.0.decrease(); }
}

impl StatMonitor for TransportMetrics {
    fn increase_tx(&mut self, n: usize) { self.transmitted.fetch_add(n, Ordering::SeqCst); }

    fn increase_rx(&mut self, n: usize) { self.received.fetch_add(n, Ordering::SeqCst); }
}

impl TransportMetrics {
    pub fn new() -> TransportMetrics {
        let received = Arc::new(AtomicUsize::new(0));
        let transmitted = Arc::new(AtomicUsize::new(0));
        let relay_counter = Counter::zero();
        let client_counter = Counter::zero();
        let remote_counter = Counter::zero();

        TransportMetrics { received, transmitted, relay_counter, client_counter, remote_counter }
    }

    #[inline]
    pub fn current_relay(&self) -> usize { self.relay_counter.current() }

    #[inline]
    pub fn accumulated_relay(&self) -> usize { self.relay_counter.accumulated() }

    #[inline]
    pub fn current_client(&self) -> usize { self.client_counter.current() }

    #[inline]
    pub fn accumulated_client(&self) -> usize { self.client_counter.accumulated() }

    #[inline]
    pub fn current_remote(&self) -> usize { self.remote_counter.current() }

    #[inline]
    pub fn accumulated_remote(&self) -> usize { self.remote_counter.accumulated() }

    #[inline]
    pub fn count_relay(&self) -> (CounterHelper, usize) {
        CounterHelper::count(self.relay_counter.clone())
    }

    #[inline]
    pub fn count_client(&self) -> (CounterHelper, usize) {
        CounterHelper::count(self.client_counter.clone())
    }

    #[inline]
    pub fn count_remote(&self) -> (CounterHelper, usize) {
        CounterHelper::count(self.remote_counter.clone())
    }
}
