use std::collections::{HashMap, HashSet};
use std::pin::Pin;

use futures::Future;
use tokio::signal::unix::{signal, SignalKind};

use crate::shutdown;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
enum ExitSignal {
    SignalTerminate,
    SignalInterrupt,
    Internal,
}

impl ExitSignal {
    fn all() -> HashSet<ExitSignal> {
        vec![ExitSignal::SignalInterrupt, ExitSignal::SignalTerminate, ExitSignal::Internal]
            .into_iter()
            .collect()
    }
}

pub type ShutdownHookFn = Box<dyn FnOnce() -> () + Send>;

pub struct LifecycleManager {
    exit_signals: HashSet<ExitSignal>,
    shutdown_slot: shutdown::ShutdownSlot,
    shutdown_hooks: HashMap<String, ShutdownHookFn>,
}

impl LifecycleManager {
    #[inline]
    pub fn new() -> (LifecycleManager, shutdown::ShutdownSignal) {
        let (shutdown_signal, shutdown_slot) = shutdown::shutdown_handle();
        (
            LifecycleManager {
                exit_signals: ExitSignal::all(),
                shutdown_hooks: HashMap::default(),
                shutdown_slot,
            },
            shutdown_signal,
        )
    }

    #[inline]
    pub fn register(&mut self, name: &str, hook: ShutdownHookFn) {
        info!("shutdown hook registered [\"{}\"]", name);
        self.shutdown_hooks.insert(name.to_owned(), hook);
    }

    #[inline]
    pub fn spawn(self) {
        let signal_handle = self.prepare();
        tokio::spawn(signal_handle);
    }

    #[inline]
    pub async fn block_on<F>(self, fut: F) -> F::Output
    where
        F: futures::Future,
    {
        let signal_handle = self.prepare();

        tokio::spawn(signal_handle);
        fut.await
    }

    async fn prepare(self) {
        let shutdown_hooks = self.shutdown_hooks;
        let mut shutdown_slot = self.shutdown_slot;
        let exit_signals =
            if self.exit_signals.is_empty() { ExitSignal::all() } else { self.exit_signals };

        let signal_receiver = {
            type SignalFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
            let mut signals: Vec<SignalFuture> = vec![];

            if exit_signals.contains(&ExitSignal::SignalTerminate) {
                signals.push(Box::pin(async move {
                    let mut term_signal = signal(SignalKind::terminate()).unwrap();
                    term_signal.recv().await;
                }));
            }

            if exit_signals.contains(&ExitSignal::SignalInterrupt) {
                signals.push(Box::pin(async move {
                    let mut int_signal = signal(SignalKind::interrupt()).unwrap();
                    int_signal.recv().await;
                }));
            }

            if exit_signals.contains(&ExitSignal::Internal) {
                signals.push(Box::pin(async move {
                    shutdown_slot.wait().await;
                }));
            }

            futures::future::select_all(signals)
        };

        info!("Waiting for shutdown signal...");
        let _ = signal_receiver.await;

        info!("Shutdown signal received");
        for (name, hook) in shutdown_hooks {
            info!("Shutdown registered hook [{}]", name);
            hook();
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::time::Duration;
    use tokio::runtime::Runtime;
    use tokio::time::delay_for;

    async fn send_signal(pid: u32, signal: ExitSignal) {
        let pid = format!("{}", pid);
        let signal = match signal {
            ExitSignal::SignalTerminate => "TERM",
            ExitSignal::SignalInterrupt => "INT",
            _ => unreachable!(),
        };

        let _ = tokio::process::Command::new("kill")
            .arg("--signal")
            .arg(signal)
            .arg(pid)
            .spawn()
            .unwrap()
            .await;
    }

    #[test]
    fn test_shutdown_slot() {
        let mut rt = Runtime::new().unwrap();
        let (mut mng, shutdown_signal) = LifecycleManager::new();
        let (loop_shutdown_signal, mut loop_shutdown_slot) = shutdown::shutdown_handle();

        mng.register(
            "loop",
            Box::new(move || {
                loop_shutdown_signal.shutdown();
            }),
        );

        let ret = rt.block_on(mng.block_on(async move {
            tokio::spawn(async move {
                delay_for(Duration::from_millis(200)).await;
                shutdown_signal.shutdown();
            });
            loop {
                loop_shutdown_slot.wait().await;
                break;
            }
        }));

        assert_eq!(ret, ());
    }

    #[test]
    fn test_int_signal() {
        let mut rt = Runtime::new().unwrap();
        let (mut mng, _shutdown_signal) = LifecycleManager::new();
        let (loop_shutdown_signal, mut loop_shutdown_slot) = shutdown::shutdown_handle();

        mng.register(
            "loop",
            Box::new(move || {
                loop_shutdown_signal.shutdown();
            }),
        );

        let pid = std::process::id();
        let ret = rt.block_on(mng.block_on(async move {
            tokio::spawn(async move {
                delay_for(Duration::from_millis(200)).await;
                send_signal(pid, ExitSignal::SignalInterrupt).await;
            });
            loop {
                loop_shutdown_slot.wait().await;
                break;
            }
        }));

        assert_eq!(ret, ());
    }

    #[test]
    fn test_term_signal() {
        let mut rt = Runtime::new().unwrap();
        let (mut mng, _shutdown_signal) = LifecycleManager::new();
        let (loop_shutdown_signal, mut loop_shutdown_slot) = shutdown::shutdown_handle();

        mng.register(
            "loop",
            Box::new(move || {
                loop_shutdown_signal.shutdown();
            }),
        );

        let pid = std::process::id();
        let ret = rt.block_on(mng.block_on(async move {
            tokio::spawn(async move {
                delay_for(Duration::from_millis(200)).await;
                send_signal(pid, ExitSignal::SignalTerminate).await;
            });
            loop {
                loop_shutdown_slot.wait().await;
                break;
            }
        }));

        assert_eq!(ret, ());
    }
}
