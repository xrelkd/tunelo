use std::{
    collections::HashSet,
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use tokio::sync::Mutex;

use crate::common::HostAddress;

#[derive(Clone, Debug)]
pub struct TransportMetrics {
    received_bytes: Arc<AtomicUsize>,
    transmitted_bytes: Arc<AtomicUsize>,
    relay_counter: Counter,
    client_counter: Counter,
    remote_counter: Counter,

    // TODO: use `destinations`
    _destinations: Arc<Mutex<HashSet<HostAddress>>>,
}

#[derive(Clone, Debug)]
pub struct Counter {
    current: Arc<AtomicUsize>,
    accumulated: Arc<AtomicUsize>,
}

impl Counter {
    #[inline]
    pub fn new(n: usize) -> Self {
        let current = Arc::new(AtomicUsize::new(n));
        let accumulated = Arc::new(AtomicUsize::new(n));
        Self { current, accumulated }
    }

    #[inline]
    pub fn zero() -> Self { Self::new(0) }

    #[inline]
    pub fn increase(&self) -> usize {
        self.accumulated.fetch_add(1, Ordering::SeqCst);
        self.current.fetch_add(1, Ordering::SeqCst)
    }

    #[inline]
    pub fn decrease(&self) -> usize { self.current.fetch_sub(1, Ordering::SeqCst) }

    #[inline]
    pub fn current(&self) -> usize { self.current.load(Ordering::Acquire) }

    #[inline]
    pub fn accumulated(&self) -> usize { self.accumulated.load(Ordering::Acquire) }
}

pub struct CounterHelper(Counter);

impl CounterHelper {
    #[inline]
    pub fn count(counter: Counter) -> (Self, usize) {
        let prev = counter.increase();
        (Self(counter), prev)
    }
}

impl Drop for CounterHelper {
    fn drop(&mut self) { self.0.decrease(); }
}

// FIXME: re-implement this
// impl StatMonitor for TransportMetrics {
//     fn increase_tx(&mut self, n: usize) { self.transmitted_bytes.fetch_add(n,
// Ordering::SeqCst); }
//
//     fn increase_rx(&mut self, n: usize) { self.received_bytes.fetch_add(n,
// Ordering::SeqCst); } }

impl Default for TransportMetrics {
    fn default() -> Self {
        let received_bytes = Arc::new(AtomicUsize::new(0));
        let transmitted_bytes = Arc::new(AtomicUsize::new(0));
        let relay_counter = Counter::zero();
        let client_counter = Counter::zero();
        let remote_counter = Counter::zero();

        let destinations = Arc::new(Mutex::new(HashSet::new()));

        Self {
            received_bytes,
            transmitted_bytes,
            relay_counter,
            client_counter,
            remote_counter,

            _destinations: destinations,
        }
    }
}

impl TransportMetrics {
    #[inline]
    pub fn new() -> Self { Self::default() }

    #[inline]
    pub fn reset(&mut self) { *self = Self::new(); }

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

impl fmt::Display for TransportMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rx: {} bytes, tx: {} bytes, client: {}/{}, relay: {}/{}, remote: {}/{}",
            self.received_bytes.load(Ordering::SeqCst),
            self.transmitted_bytes.load(Ordering::SeqCst),
            self.current_client(),
            self.accumulated_client(),
            self.current_relay(),
            self.accumulated_relay(),
            self.current_remote(),
            self.accumulated_remote(),
        )
    }
}
