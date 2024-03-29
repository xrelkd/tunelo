use crate::{
    checker::{Error, ReportError},
    client::ProxyConnector,
    common::{ProxyHost, ProxyStrategy},
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LivenessProber;

impl Default for LivenessProber {
    fn default() -> Self { Self }
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

#[derive(Clone, Debug, Default)]
pub struct LivenessProberReport {
    pub alive: bool,
    pub error: Option<ReportError>,
}

impl LivenessProberReport {
    #[inline]
    #[must_use]
    pub fn timeout() -> Self { Self { alive: false, error: Some(ReportError::Timeout) } }

    #[inline]
    #[must_use]
    pub fn has_error(&self) -> bool { self.error.is_some() }
}
