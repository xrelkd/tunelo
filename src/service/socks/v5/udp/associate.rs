use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use tokio::{
    net::UdpSocket,
    sync::{mpsc, Mutex},
};

use crate::{protocol::socks::v5::Datagram, service::socks::Error, transport::Resolver};

pub struct UdpAssociate {
    tx: Mutex<mpsc::Sender<Datagram>>,
    closed: Arc<AtomicBool>,
}

impl Drop for UdpAssociate {
    fn drop(&mut self) { self.closed.store(true, Ordering::Release); }
}

impl UdpAssociate {
    #[inline]
    pub async fn send_to(&self, datagram: Datagram) -> bool {
        match self.tx.lock().await.send(datagram).await {
            Ok(_) => true,
            Err(err) => {
                error!("Failed to send packet, error: {:?}", err);
                false
            }
        }
    }

    pub async fn new(
        client_addr: SocketAddr,
        mut response_tx: mpsc::Sender<(SocketAddr, Datagram)>,
        resolver: Arc<dyn Resolver>,
    ) -> Result<UdpAssociate, Error> {
        let (mut socket_recv, mut socket_send) = {
            let local_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
            let remote_socket = UdpSocket::bind(&local_addr).await?;
            remote_socket.split()
        };

        let (tx, mut rx) = mpsc::channel::<Datagram>(1024);
        let closed = Arc::new(AtomicBool::new(false));

        // local to remote
        tokio::spawn({
            async move {
                while let Some(datagram) = rx.recv().await {
                    use crate::common::HostAddress;
                    let remote_host = match datagram.destination_address() {
                        HostAddress::Socket(addr) => addr.clone(),
                        HostAddress::DomainName(host, port) => {
                            match resolver.resolve(&host).await {
                                Ok(addrs) => {
                                    if addrs.is_empty() {
                                        return;
                                    }
                                    SocketAddr::new(addrs[0], *port)
                                }
                                Err(_err) => {
                                    warn!(
                                        "Failed to resolve host address: {}",
                                        datagram.destination_address()
                                    );
                                    return;
                                }
                            }
                        }
                    };

                    match socket_send.send_to(datagram.data(), &remote_host).await {
                        Ok(n) => {
                            debug!(
                                "Send packet to remote host {} with {} bytes",
                                remote_host.to_string(),
                                n
                            );
                        }
                        Err(err) => {
                            warn!(
                                "Failed to send packet to remote host: {}, error: {:?}",
                                remote_host, err
                            );
                            break;
                        }
                    };
                }
            }
        });

        // remote to local
        tokio::spawn({
            let closed = closed.clone();
            async move {
                let mut buf = [0u8; 1024];
                while !closed.load(Ordering::Acquire) {
                    match socket_recv.recv_from(&mut buf).await {
                        Ok((n, remote_addr)) => {
                            info!(
                                "Received packet with {} bytes from remote host {}",
                                n, remote_addr
                            );

                            let datagram = Datagram::new(0, remote_addr.into(), buf[..n].to_vec());
                            if let Err(err) = response_tx.send((client_addr, datagram)).await {
                                warn!(
                                    "Failed to send packet to remote host: {}, error: {:?}",
                                    remote_addr, err
                                );
                                break;
                            }
                        }
                        Err(err) => {
                            warn!("Failed to receive packet, error: {:?}", err);
                            break;
                        }
                    }
                }
            }
        });

        Ok(UdpAssociate { tx: Mutex::new(tx), closed })
    }
}
