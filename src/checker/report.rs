use crate::{
    checker::prober::{BasicProberReport, HttpProberReport, LivenessProberReport, ProberReport},
    common::ProxyHost,
};

#[derive(Debug, Clone)]
pub struct TaskReport {
    pub proxy_server: ProxyHost,
    pub liveness_report: LivenessProberReport,
    pub prober_reports: Vec<ProberReport>,
}

impl TaskReport {
    #[must_use]
    pub const fn is_proxy_server_alive(&self) -> bool { self.liveness_report.alive }

    #[must_use]
    pub const fn liveness_report(&self) -> &LivenessProberReport { &self.liveness_report }

    pub fn basic_reports(&self) -> impl Iterator<Item = &BasicProberReport> {
        self.prober_reports.iter().filter_map(|p| match p {
            ProberReport::Basic(p) => Some(p),
            _ => None,
        })
    }

    pub fn http_reports(&self) -> impl Iterator<Item = &HttpProberReport> {
        self.prober_reports.iter().filter_map(|p| match p {
            ProberReport::Http(p) => Some(p),
            _ => None,
        })
    }

    #[must_use]
    pub fn basic_report_count(&self) -> usize { self.basic_reports().count() }

    #[must_use]
    pub fn http_report_count(&self) -> usize { self.http_reports().count() }
}
