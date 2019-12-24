use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time;

use futures::FutureExt;

use lru_time_cache::LruCache;

use crate::protocol::socks::{v5::Datagram, Error};
use crate::service::socks::v5::udp::{UdpAssociate, UdpAssociateCache};
use crate::shutdown;
use crate::transport::Resolver;

pub struct UdpServer {
    local_addr: SocketAddr,
    cache: UdpAssociateCache,
    resolver: Arc<dyn Resolver>,
    shutdown_slot: shutdown::ShutdownSlot,
}

impl UdpServer {
    pub fn new(
        local_addr: SocketAddr,
        udp_associate_cache: UdpAssociateCache,
        resolver: Arc<dyn Resolver>,
    ) -> (UdpServer, shutdown::ShutdownSignal) {
        let (shutdown_signal, shutdown_slot) = shutdown::shutdown_handle();
        (
            UdpServer { local_addr, cache: udp_associate_cache, resolver, shutdown_slot },
            shutdown_signal,
        )
    }

    pub async fn serve(self) -> Result<(), Error> {
        info!("Starting UDP server for UDP associate at {}", self.local_addr);
        let udp_socket = UdpSocket::bind(&self.local_addr).await?;
        let mut shutdown_slot = self.shutdown_slot;
        let (mut udp_recv, mut udp_send) = udp_socket.split();

        // FIXME buffer size
        let (pkt_tx, mut pkt_rx) = mpsc::channel::<(SocketAddr, Datagram)>(1024);

        tokio::spawn(async move {
            while let Some((client_addr, datagram)) = pkt_rx.recv().await {
                if let Err(err) = udp_send.send_to(&datagram.into_bytes(), &client_addr).await {
                    error!("UDP packet send failed, error: {:?}", err);
                    break;
                }
            }
        });

        let mut udp_associates: LruCache<String, UdpAssociate> =
            LruCache::with_expiry_duration(Duration::from_secs(10));
        let timeout = Duration::from_secs(5);
        let mut buf = [0u8; 2048];

        loop {
            let (buf_len, client_addr) = futures::select! {
                _ = shutdown_slot.wait().fuse() => break,
                res = time::timeout(timeout, udp_recv.recv_from(&mut buf)).fuse() => {
                    match res {
                        Ok(Ok((n, client_addr))) => {
                            info!("Received {} byte(s) from {}", n, client_addr);
                            (n, client_addr)
                        },
                        Ok(Err(err)) => {
                            warn!(
                                "Failed to receive data from local UDP listener {}, error: {:?}",
                                self.local_addr, err
                            );
                            break;
                        }
                        Err(_err) => {
                            debug!("Remove expired UDP associate objects");
                            let _ = udp_associates.iter();
                            continue;
                        }
                    }
                }
            };

            if buf_len == 0 {
                continue;
            }

            let datagram = match Datagram::from_bytes(&buf[0..buf_len]) {
                Ok(datagram) => datagram,
                Err(err) => {
                    info!("Failed to parse packet from client: {}, error: {:?}", client_addr, err);
                    continue;
                }
            };

            match (
                self.cache.contains(&client_addr.into()).await,
                udp_associates.get(&client_addr.to_string()),
            ) {
                (true, Some(associate)) => {
                    associate.send_to(datagram).await;
                }
                (true, None) => {
                    match UdpAssociate::new(
                        client_addr.clone(),
                        pkt_tx.clone(),
                        self.resolver.clone(),
                    )
                    .await
                    {
                        Ok(associate) => {
                            associate.send_to(datagram).await;
                            udp_associates.insert(client_addr.to_string(), associate);
                        }
                        Err(err) => {
                            warn!(
                                "Failed to create UDP associate for client {}, error: {:?}",
                                client_addr, err
                            );

                            self.cache.remove(&client_addr.into()).await;
                        }
                    };
                }
                (false, _) => {}
            }
        }

        info!("UDP server {} is stopped", self.local_addr);
        Ok(())
    }
}
