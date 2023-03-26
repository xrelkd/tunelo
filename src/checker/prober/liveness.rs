use crate::{
    checker::{Error, ReportError},
    client::ProxyConnector,
    common::{ProxyHost, ProxyStrategy},
};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LivenessProber;

impl Default for LivenessProber {
    fn default() -> LivenessProber { LivenessProber }
}

impl LivenessProber {
    #[inline]
    pub async fn probe(self, proxy_server: &ProxyHost) -> LivenessProberReport {
        let mut report = LivenessProberReport::default();
        let alive =
            ProxyConnector::probe_liveness(&ProxyStrategy::Single(proxy_server.clone()), None)
                .await;

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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LivenessProberReport {
    pub alive: bool,
    pub error: Option<ReportError>,
}

impl LivenessProberReport {
    #[inline]
    pub fn timeout() -> Self { Self { alive: false, error: Some(ReportError::Timeout) } }

    #[inline]
    pub fn has_error(&self) -> bool { self.error.is_some() }
}
