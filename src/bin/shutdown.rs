use tokio::sync::mpsc;

#[derive(Clone)]
pub struct ShutdownSender(mpsc::Sender<()>);

pub struct ShutdownReceiver(mpsc::Receiver<()>);

pub fn shutdown_handle() -> (ShutdownSender, ShutdownReceiver) {
    let (sender, receiver) = mpsc::channel(1);
    (ShutdownSender(sender), ShutdownReceiver(receiver))
}

impl ShutdownSender {
    pub fn shutdown(self) {
        drop(self.0);
    }
}

impl ShutdownReceiver {
    pub async fn wait(&mut self) {
        self.0.recv().await;
    }
}
