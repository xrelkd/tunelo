use crate::{
    checker::{Error, ReportError},
    client::ProxyConnector,
    common::{ProxyHost, ProxyStrategy},
};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LivenessProber;

impl LivenessProber {
    #[inline]
    pub fn new() -> LivenessProber { LivenessProber }

    #[inline]
    pub async fn probe(self, proxy_server: &ProxyHost) -> LivenessProberReport {
        let mut report = LivenessProberReport::default();
        let alive =
            ProxyConnector::probe_liveness(&ProxyStrategy::Single(proxy_server.clone())).await;
        match alive {
            Ok(alive) => {
                report.alive = alive;
                report.error = None;
            }
            Err(source) => {
                report.alive = false;
                report.error = Some(Error::ConnectProxyServer { source }.into());
            }
        };

        report
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LivenessProberReport {
    pub alive: bool,
    pub error: Option<ReportError>,
}

impl LivenessProberReport {
    pub fn has_error(&self) -> bool { self.error.is_some() }
}

impl Default for LivenessProberReport {
    fn default() -> Self { LivenessProberReport { alive: false, error: None } }
}
