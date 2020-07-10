use crate::{
    checker::{Error, ReportError},
    client::ProxyStream,
    common::{HostAddress, ProxyHost},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BasicProberReport {
    pub destination_reachable: bool,
    pub destination: Option<HostAddress>,
    pub error: Option<ReportError>,
}

impl BasicProberReport {
    #[inline]
    pub fn timeout(destination: HostAddress) -> BasicProberReport {
        BasicProberReport {
            destination_reachable: false,
            destination: Some(destination),
            error: Some(ReportError::Timeout),
        }
    }

    #[inline]
    pub fn has_error(&self) -> bool { self.error.is_some() }
}

impl Default for BasicProberReport {
    fn default() -> Self {
        BasicProberReport { destination: None, destination_reachable: false, error: None }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BasicProber {
    destination: HostAddress,
}

impl BasicProber {
    #[inline]
    pub fn new(destination: HostAddress) -> BasicProber { BasicProber { destination } }

    #[inline]
    pub async fn probe(
        self,
        proxy_server: &ProxyHost,
        report: &mut BasicProberReport,
    ) -> Result<(), Error> {
        report.destination = Some(self.destination.clone());
        let stream = ProxyStream::connect_with_proxy(&proxy_server, &self.destination)
            .await
            .map_err(|source| Error::ConnectProxyServer { source })?;

        report.destination_reachable = true;

        let stream = stream.into_inner();
        stream.shutdown(std::net::Shutdown::Both).map_err(|source| Error::Shutdown { source })?;

        Ok(())
    }

    #[inline]
    pub fn destination(&self) -> &HostAddress { &self.destination }
}
