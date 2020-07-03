use tokio::sync::mpsc;

#[derive(Clone)]
pub struct ShutdownSignal(mpsc::Sender<()>);

pub struct ShutdownSlot(mpsc::Receiver<()>);

pub fn shutdown_handle() -> (ShutdownSignal, ShutdownSlot) {
    let (sender, receiver) = mpsc::channel(1);
    (ShutdownSignal(sender), ShutdownSlot(receiver))
}

impl ShutdownSignal {
    pub fn shutdown(self) { drop(self.0); }
}

impl ShutdownSlot {
    pub async fn wait(&mut self) { self.0.recv().await; }
}

pub struct JoinHandle<T> {
    shutdown_signal: ShutdownSignal,
    join_handle: tokio::task::JoinHandle<T>,
}

impl<T> JoinHandle<T> {
    pub fn new(
        shutdown_signal: ShutdownSignal,
        join_handle: tokio::task::JoinHandle<T>,
    ) -> JoinHandle<T> {
        JoinHandle { shutdown_signal, join_handle }
    }

    pub async fn shutdown_and_wait(self) {
        self.shutdown_signal.shutdown();
        let _ = self.join_handle.await;
    }
}
