use std::sync::Arc;

use tokio::sync::Mutex;

use crate::common::{HostAddress, ProxyHost};

#[derive(Debug)]
pub enum Error {}

#[derive(Debug)]
pub struct Report {
    task_reports: Vec<TaskReport>,
}

impl From<Vec<TaskReport>> for Report {
    fn from(task_reports: Vec<TaskReport>) -> Report {
        Report { task_reports }
    }
}

#[derive(Debug)]
pub struct TaskReport {}

#[derive(Debug)]
pub struct ProxyChecker {
    parallel_count: usize,
    proxy_servers: Vec<ProxyHost>,
    tasks: Arc<Mutex<Vec<Task>>>,
}

impl ProxyChecker {
    pub fn new(proxy_servers: Vec<ProxyHost>, target_hosts: Vec<HostAddress>) -> ProxyChecker {
        Self::with_parallel(10, proxy_servers, target_hosts)
    }

    pub fn with_parallel(
        parallel_count: usize,
        proxy_servers: Vec<ProxyHost>,
        target_hosts: Vec<HostAddress>,
    ) -> ProxyChecker {
        let target_hosts = Arc::new(target_hosts);
        let tasks: Vec<_> = proxy_servers
            .iter()
            .enumerate()
            .rev()
            .map(|(id, proxy_server)| Task::new(id, proxy_server.clone(), target_hosts.clone()))
            .collect();

        let tasks = Arc::new(Mutex::new(tasks));
        ProxyChecker { parallel_count, tasks, proxy_servers }
    }

    pub async fn run(self) -> Result<Report, Error> {
        let runners = (0..self.parallel_count).fold(
            Vec::with_capacity(self.parallel_count),
            |mut runners, id| {
                let runner = TaskRunner::new(id, self.tasks.clone());
                runners.push(runner.run());
                runners
            },
        );

        let reports = futures::future::join_all(runners).await.into_iter().fold(
            Vec::with_capacity(self.proxy_servers.len()),
            |mut all_reports, reports| {
                all_reports.extend(reports);
                all_reports
            },
        );

        Ok(Report::from(reports))
    }
}

struct TaskRunner {
    id: usize,
    queue: Arc<Mutex<Vec<Task>>>,
}

impl TaskRunner {
    fn new(id: usize, queue: Arc<Mutex<Vec<Task>>>) -> TaskRunner {
        TaskRunner { id, queue }
    }

    async fn run(self) -> Vec<TaskReport> {
        info!("TaskRunner {} is running", self.id);
        let mut reports = Vec::with_capacity(16);

        loop {
            let task = {
                match self.queue.lock().await.pop() {
                    Some(task) => task,
                    None => break,
                }
            };

            info!("TaskRunner {}: testing {} {:?}", self.id, task.id, task.proxy_server);
            reports.push(task.run().await);
        }

        info!("TaskRunner {} is finished", self.id);
        reports
    }
}

#[derive(Debug)]
struct Task {
    id: usize,
    proxy_server: ProxyHost,
    target_hosts: Arc<Vec<HostAddress>>,
}

impl Task {
    fn new(id: usize, proxy_server: ProxyHost, target_hosts: Arc<Vec<HostAddress>>) -> Task {
        Task { id, proxy_server, target_hosts }
    }

    async fn run(self) -> TaskReport {
        tokio::time::delay_for(std::time::Duration::from_millis(200)).await;
        TaskReport {}
    }
}
