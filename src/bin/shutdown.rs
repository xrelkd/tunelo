use tokio::sync::broadcast;

#[derive(Clone)]
pub struct ShutdownSender(broadcast::Sender<()>);

pub struct ShutdownReceiver(broadcast::Receiver<()>);

pub fn new() -> (ShutdownSender, ShutdownReceiver) {
    let (sender, receiver) = broadcast::channel(1);
    (ShutdownSender(sender), ShutdownReceiver(receiver))
}

impl ShutdownSender {
    pub fn subscribe(&self) -> ShutdownReceiver {
        let receiver = self.0.subscribe();
        ShutdownReceiver(receiver)
    }

    pub fn shutdown(self) { drop(self.0); }
}

impl ShutdownReceiver {
    pub async fn wait(&mut self) { let _ = self.0.recv().await; }
}
