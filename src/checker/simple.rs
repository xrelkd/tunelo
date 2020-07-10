use std::{collections::HashSet, time::Duration};

use crate::common::ProxyHost;

pub use crate::checker::{
    error::{Error, ReportError},
    prober::{
        BasicProber, BasicProberReport, HttpMethod, HttpProber, HttpProberReport, LivenessProber,
        LivenessProberReport, Prober, ProberReport,
    },
    report::TaskReport,
};

#[derive(Debug, Clone)]
pub struct SimpleProxyChecker {
    proxy_server: ProxyHost,
    probers: HashSet<Prober>,
}

impl SimpleProxyChecker {
    #[inline]
    pub fn new(proxy_server: ProxyHost) -> Self {
        SimpleProxyChecker { proxy_server, probers: HashSet::new() }
    }

    #[inline]
    pub fn with_probers(proxy_server: ProxyHost, probers: &[Prober]) -> Self {
        let probers = probers.iter().cloned().collect();
        SimpleProxyChecker { proxy_server, probers }
    }

    #[inline]
    pub fn add_prober(&mut self, prober: Prober) { self.probers.insert(prober); }

    #[inline]
    pub async fn prepare(self, timeout: Option<Duration>) -> (ProxyHost, Vec<Prober>, TaskReport) {
        let liveness_report = match timeout {
            None => self.check_liveness().await,
            Some(t) => tokio::time::timeout(t, self.check_liveness())
                .await
                .unwrap_or(LivenessProberReport::timeout()),
        };

        let mut probers = self.probers;

        // remove redundant default LivenessProber
        probers.remove(&LivenessProber::default().into());

        let mut probers: Vec<_> = probers.iter().cloned().collect();
        probers.sort();

        let task_report = TaskReport {
            proxy_server: self.proxy_server.clone(),
            liveness_report: liveness_report.into(),
            prober_reports: vec![],
        };

        (self.proxy_server, probers, task_report)
    }

    pub async fn check_liveness(&self) -> LivenessProberReport {
        let liveness_prober = LivenessProber::default();
        liveness_prober.probe(&self.proxy_server).await
    }

    pub async fn run(self, timeout: Option<Duration>) -> TaskReport {
        let (proxy_server, probers, mut task_report) = self.prepare(timeout).await;

        if !task_report.is_proxy_server_alive() {
            return task_report;
        }

        for prober in probers {
            let report = prober.probe(&proxy_server, timeout).await;
            task_report.prober_reports.push(report);
        }

        task_report.prober_reports.sort();
        task_report
    }

    pub async fn run_parallel(self, timeout_per_probe: Option<Duration>) -> TaskReport {
        let (proxy_server, probers, mut task_report) = self.prepare(timeout_per_probe).await;

        if !task_report.is_proxy_server_alive() {
            return task_report;
        }

        let futs: Vec<_> = probers
            .into_iter()
            .map(|checker| checker.probe(&proxy_server, timeout_per_probe))
            .collect();

        let mut reports: Vec<_> = futures::future::join_all(futs).await.into_iter().collect();
        task_report.prober_reports.append(&mut reports);
        task_report.prober_reports.sort();
        task_report
    }

    #[inline]
    pub fn proxy_server(&self) -> &ProxyHost { &self.proxy_server }
}
