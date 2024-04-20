use std::time::Duration;

use crate::common::ProxyHost;

mod basic;
mod http;
mod liveness;

pub use self::{
    basic::{BasicProber, BasicProberReport},
    http::{HttpMethod, HttpProber, HttpProberReport},
    liveness::{LivenessProber, LivenessProberReport},
};

#[derive(Clone, Debug, Hash)]
pub enum Prober {
    Liveness(LivenessProber),
    Basic(BasicProber),
    Http(HttpProber),
}

impl Prober {
    #[must_use]
    pub const fn precedence(&self) -> usize {
        match self {
            Self::Liveness(_) => 0,
            Self::Basic(_) => 1,
            Self::Http(_) => 2,
        }
    }

    fn timeout_report(&self) -> ProberReport {
        match self {
            Self::Liveness(_) => LivenessProberReport::timeout().into(),
            Self::Basic(p) => BasicProberReport::timeout(p.destination().clone()).into(),
            Self::Http(p) => HttpProberReport::timeout(p.method(), p.url().clone()).into(),
        }
    }

    pub async fn probe(self, proxy_server: &ProxyHost, timeout: Option<Duration>) -> ProberReport {
        match timeout {
            Some(timeout) => {
                let timeout_report = self.timeout_report();
                match tokio::time::timeout(timeout, self.probe_internal(proxy_server)).await {
                    Ok(r) => r,
                    Err(_err) => timeout_report,
                }
            }
            None => self.probe_internal(proxy_server).await,
        }
    }

    async fn probe_internal(self, proxy_server: &ProxyHost) -> ProberReport {
        match self {
            Self::Liveness(prober) => ProberReport::Liveness(prober.probe(proxy_server).await),
            Self::Basic(prober) => {
                let mut report = BasicProberReport::default();
                match prober.probe(proxy_server, &mut report).await {
                    Ok(_) => ProberReport::Basic(report),
                    Err(err) => {
                        report.error = Some(err.into());
                        ProberReport::Basic(report)
                    }
                }
            }
            Self::Http(prober) => {
                let mut report = HttpProberReport::default();
                match prober.probe(proxy_server, &mut report).await {
                    Ok(_) => ProberReport::Http(report),
                    Err(err) => {
                        report.error = Some(err.into());
                        ProberReport::Http(report)
                    }
                }
            }
        }
    }
}

macro_rules! impl_from_prober {
    ($prober:ty, $field:ident) => {
        impl From<$prober> for Prober {
            fn from(prober: $prober) -> Prober { Prober::$field(prober) }
        }
    };
}

impl_from_prober!(LivenessProber, Liveness);
impl_from_prober!(BasicProber, Basic);
impl_from_prober!(HttpProber, Http);

// impl Ord for Prober {
//     fn cmp(&self, other: &Prober) -> std::cmp::Ordering {
//         self.precedence().cmp(&other.precedence())
//     }
// }
//
// impl PartialOrd for Prober {
//     fn partial_cmp(&self, other: &Prober) -> Option<std::cmp::Ordering> {
//         self.precedence().partial_cmp(&other.precedence())
//     }
// }

#[derive(Clone, Debug)]
pub enum ProberReport {
    Liveness(LivenessProberReport),
    Basic(BasicProberReport),
    Http(HttpProberReport),
}

impl ProberReport {
    #[must_use]
    pub const fn precedence(&self) -> usize {
        match self {
            Self::Liveness(_) => 0,
            Self::Basic(_) => 1,
            Self::Http(_) => 2,
        }
    }

    #[must_use]
    pub fn has_error(&self) -> bool {
        match self {
            Self::Liveness(r) => r.has_error(),
            Self::Basic(r) => r.has_error(),
            Self::Http(r) => r.has_error(),
        }
    }
}

macro_rules! impl_from_prober_report {
    ($prober:ty, $field:ident) => {
        impl From<$prober> for ProberReport {
            fn from(prober: $prober) -> ProberReport { ProberReport::$field(prober) }
        }
    };
}

impl_from_prober_report!(LivenessProberReport, Liveness);
impl_from_prober_report!(BasicProberReport, Basic);
impl_from_prober_report!(HttpProberReport, Http);

// impl Ord for ProberReport {
//     fn cmp(&self, other: &ProberReport) -> std::cmp::Ordering {
//         self.precedence().cmp(&other.precedence())
//     }
// }
//
// impl PartialOrd for ProberReport {
//     fn partial_cmp(&self, other: &ProberReport) -> Option<std::cmp::Ordering>
// {         self.precedence().partial_cmp(&other.precedence())
//     }
// }
