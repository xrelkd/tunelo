use std::sync::atomic;

use futures::FutureExt;
use tokio::signal::unix::{signal, SignalKind};

use crate::SHUTDOWN;

pub type ShutdownHookFn = Box<dyn FnOnce() -> () + Send>;

#[allow(clippy::never_loop)]
async fn run(shutdown_hook: ShutdownHookFn) {
    let mut term_signal = signal(SignalKind::terminate()).unwrap();
    let mut int_signal = signal(SignalKind::interrupt()).unwrap();

    let mut shutdown_hook = Some(shutdown_hook);

    loop {
        loop {
            futures::select! {
                _ = term_signal.recv().fuse() => {
                    info!("SIGTERM received!");
                    break;
                },
                _ = int_signal.recv().fuse() => {
                    info!("SIGINT received!");
                    break;
                },
            }
        }

        if SHUTDOWN.load(atomic::Ordering::SeqCst) {
            info!("Terminating process!");
            std::process::abort();
        } else {
            info!("Shutting down cleanly. Interrupt again to shut down immediately.");
            SHUTDOWN.store(true, atomic::Ordering::SeqCst);
            let shutdown_hook = shutdown_hook.take().unwrap();
            shutdown_hook();
        }
    }
}

pub fn start(shutdown_hook: ShutdownHookFn) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn(run(shutdown_hook))
}
