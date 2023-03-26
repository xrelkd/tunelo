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

#[derive(Debug, Clone, Hash)]
pub enum Prober {
    Liveness(LivenessProber),
    Basic(BasicProber),
    Http(HttpProber),
}

impl Prober {
    pub fn precedence(&self) -> usize {
        match self {
            Prober::Liveness(_) => 0,
            Prober::Basic(_) => 1,
            Prober::Http(_) => 2,
        }
    }

    fn timeout_report(&self) -> ProberReport {
        match self {
            Prober::Liveness(_) => LivenessProberReport::timeout().into(),
            Prober::Basic(p) => BasicProberReport::timeout(p.destination().clone()).into(),
            Prober::Http(p) => HttpProberReport::timeout(p.method(), p.url().clone()).into(),
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
            Prober::Liveness(prober) => ProberReport::Liveness(prober.probe(proxy_server).await),
            Prober::Basic(prober) => {
                let mut report = BasicProberReport::default();
                match prober.probe(proxy_server, &mut report).await {
                    Ok(_) => ProberReport::Basic(report),
                    Err(err) => {
                        report.error = Some(err.into());
                        ProberReport::Basic(report)
                    }
                }
            }
            Prober::Http(prober) => {
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
    pub fn precedence(&self) -> usize {
        match self {
            ProberReport::Liveness(_) => 0,
            ProberReport::Basic(_) => 1,
            ProberReport::Http(_) => 2,
        }
    }

    pub fn has_error(&self) -> bool {
        match self {
            ProberReport::Liveness(r) => r.has_error(),
            ProberReport::Basic(r) => r.has_error(),
            ProberReport::Http(r) => r.has_error(),
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
