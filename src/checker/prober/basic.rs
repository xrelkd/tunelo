use snafu::ResultExt;

use crate::{
    checker::{error, Error, ReportError},
    client::ProxyStream,
    common::{HostAddress, ProxyHost},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BasicProberReport {
    pub destination_reachable: bool,
    pub destination: Option<HostAddress>,
    pub error: Option<ReportError>,
}

impl BasicProberReport {
    #[inline]
    pub fn timeout(destination: HostAddress) -> Self {
        Self {
            destination_reachable: false,
            destination: Some(destination),
            error: Some(ReportError::Timeout),
        }
    }

    #[inline]
    pub fn has_error(&self) -> bool { self.error.is_some() }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BasicProber {
    destination: HostAddress,
}

impl BasicProber {
    #[inline]
    pub fn new(destination: HostAddress) -> Self { Self { destination } }

    #[inline]
    pub async fn probe(
        self,
        proxy_server: &ProxyHost,
        report: &mut BasicProberReport,
    ) -> Result<(), Error> {
        report.destination = Some(self.destination.clone());
        let stream = ProxyStream::connect_with_proxy(proxy_server, &self.destination)
            .await
            .context(error::ConnectProxyServer)?;

        report.destination_reachable = true;

        let stream = stream.into_inner();
        stream.shutdown(std::net::Shutdown::Both).context(error::Shutdown)?;

        Ok(())
    }

    #[inline]
    pub fn destination(&self) -> &HostAddress { &self.destination }
}
