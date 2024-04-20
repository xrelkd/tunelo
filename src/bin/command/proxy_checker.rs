use std::{
    convert::TryInto,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use clap::Args;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tunelo::{
    checker::{BasicProber, HttpProber, LivenessProber, Prober, SimpleProxyChecker, TaskReport},
    common::{HostAddress, ProxyHost},
};
use url::Url;

use crate::error::{self, Error};

pub async fn run<P: AsRef<Path>>(options: Options, config_file: Option<P>) -> Result<(), Error> {
    let output_path = options.output_path.clone();
    let mut config = match config_file {
        Some(path) => Config::load(path)?.merge(options),
        None => Config::default().merge(options),
    };

    if let Some(file) = config.proxy_server_file {
        let file = ProxyServerFile::load(file)?;
        config.proxy_servers = file.proxy_servers;
    }

    if config.proxy_servers.is_empty() {
        return Err(Error::NoProxyHostProvided);
    }

    let probers: Vec<_> =
        config.probers.into_iter().filter_map(|prober| prober.try_into().ok()).collect();
    if probers.is_empty() {
        return Err(Error::NoProxyProberProvided);
    }

    let checkers: Vec<_> = config
        .proxy_servers
        .into_iter()
        .map(|proxy_host| SimpleProxyChecker::with_probers(proxy_host, &probers))
        .collect();

    let reports = {
        let max_timeout_per_probe = config.max_timeout_per_probe;
        let report_futs = checkers.into_iter().map(|checker| async {
            println!("Checking proxy server: {}", checker.proxy_server());
            checker.run_parallel(max_timeout_per_probe).await
        });

        futures::future::join_all(report_futs).await
    };

    write_reports_to(&mut std::io::stdout(), &reports)
        .context(error::WriteProxyCheckerReportSnafu)?;

    if let Some(ref path) = &output_path {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)
            .context(error::WriteProxyHostsSnafu)?;

        write_available_proxy_servers(&mut file, &reports).context(error::WriteProxyHostsSnafu)?;
    }

    Ok(())
}

fn write_available_proxy_servers<W>(
    writer: &mut W,
    reports: &[TaskReport],
) -> Result<(), std::io::Error>
where
    W: std::io::Write,
{
    let proxy_servers: Vec<_> = reports
        .iter()
        .filter_map(|r| if r.is_proxy_server_alive() { Some(r.proxy_server.clone()) } else { None })
        .collect();

    let file = ProxyServerFile { proxy_servers };
    writeln!(writer, "{}", toml::to_string(&file).expect("ProxyServerFile is serializable"))?;

    Ok(())
}

fn write_reports_to<W>(writer: &mut W, reports: &[TaskReport]) -> Result<(), std::io::Error>
where
    W: std::io::Write,
{
    use comfy_table::{ContentArrangement, Table};

    for report in reports {
        {
            let mut table = Table::new();
            table
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec!["Server", "Type", "Host", "Port", "Alive", "Error"]);

            let r = report.liveness_report();

            let alive = r.alive.to_string();
            let err = r.error.as_ref().map(ToString::to_string).unwrap_or_default();
            let proxy_server = &report.proxy_server;
            let proxy_server_url = proxy_server.to_string();
            table.add_row(vec![
                proxy_server_url,
                proxy_server.proxy_type_str().to_owned(),
                proxy_server.host().to_owned(),
                proxy_server.port().to_string(),
                alive,
                err,
            ]);

            writeln!(writer, "{table}")?;
        }

        if report.basic_report_count() != 0 {
            let mut table = Table::new();
            table.set_content_arrangement(ContentArrangement::Dynamic).set_header(vec![
                "Basic Probe",
                "Destination",
                "Connected",
                "Error",
            ]);

            for r in report.basic_reports() {
                let destination_reachable = r.destination_reachable.to_string();
                let destination =
                    r.destination.as_ref().map(ToString::to_string).unwrap_or_default();
                let err = r.error.as_ref().map(ToString::to_string).unwrap_or_default();
                table.add_row(vec![String::new(), destination, destination_reachable, err]);
            }

            writeln!(writer, "{table}")?;
        }

        if report.http_report_count() != 0 {
            let mut table = Table::new();
            table.set_content_arrangement(ContentArrangement::Dynamic).set_header(vec![
                "HTTP Probe",
                "Method ",
                "URL",
                "Resp. Code",
                "Error",
            ]);

            for r in report.http_reports() {
                let method = r.method.as_ref().map(ToString::to_string).unwrap_or_default();
                let response_code =
                    r.response_code.as_ref().map_or_else(|| "N/A".to_owned(), ToString::to_string);
                let url = r.url.as_ref().map(ToString::to_string).unwrap_or_default();
                let err = r.error.as_ref().map(ToString::to_string).unwrap_or_default();

                table.add_row(vec![String::new(), method, url, response_code, err]);
            }

            writeln!(writer, "{table}")?;
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    proxy_servers: Vec<ProxyHost>,
    proxy_server_file: Option<PathBuf>,
    probers: Vec<ProberConfig>,
    max_timeout_per_probe: Option<Duration>,
}

impl Config {
    impl_config_load!(Config);

    fn merge(mut self, opts: Options) -> Self {
        if !opts.proxy_servers.is_empty() {
            self.proxy_servers = opts.proxy_servers;
        }

        if !opts.probers.is_empty() {
            self.probers = opts.probers;
        }

        if opts.proxy_server_file.is_some() {
            self.proxy_server_file = opts.proxy_server_file;
        }

        if let Some(ms) = opts.max_timeout_per_probe {
            self.max_timeout_per_probe = Some(Duration::from_millis(ms));
        }

        self
    }
}

impl Default for Config {
    fn default() -> Self {
        let probers = vec![
            ProberConfig::Liveness,
            ProberConfig::HttpGet {
                url: "https://httpbin.org/ip".to_owned(),
                expected_response_code: 200,
            },
        ];
        Self {
            proxy_servers: vec![],
            proxy_server_file: None,
            probers,
            max_timeout_per_probe: Some(Duration::from_millis(1500)),
        }
    }
}

#[derive(Args, Debug)]
pub struct Options {
    #[arg(long = "proxy-servers", short = 's', help = "Proxy server list")]
    proxy_servers: Vec<ProxyHost>,

    #[arg(long = "file", short = 'f', help = "Proxy server list file")]
    proxy_server_file: Option<PathBuf>,

    #[arg(long = "output-file", short = 'o')]
    output_path: Option<PathBuf>,

    #[arg(long = "probers", short = 'p', help = "Proxy probers")]
    probers: Vec<ProberConfig>,

    #[arg(long = "max-timeout-per-probe", help = "Max timeout per probe in millisecond")]
    max_timeout_per_probe: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", tag = "prober")]
pub enum ProberConfig {
    Liveness,
    Basic { destination_address: HostAddress },
    HttpGet { url: String, expected_response_code: u16 },
    HttpHead { url: String, expected_response_code: u16 },
    HttpDelete { url: String, expected_response_code: u16 },
}

impl FromStr for ProberConfig {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split(',').map(str::trim).collect();

        let probe_type = parts[0].to_lowercase();
        if probe_type.starts_with("http") {
            if parts.len() < 3 {
                return Err(Error::InvalidProxyProber { prober: s.to_owned() });
            }

            let url = parts[1].to_owned();
            let expected_response_code =
                parts[2].parse().map_err(|_| Error::InvalidProxyProber { prober: s.to_owned() })?;

            let prober = match probe_type.as_str() {
                "http-get" => Self::HttpGet { url, expected_response_code },
                "http-head" => Self::HttpHead { url, expected_response_code },
                "http-delete" => Self::HttpDelete { url, expected_response_code },
                _ => {
                    return Err(Error::InvalidProxyProber { prober: s.to_owned() });
                }
            };
            return Ok(prober);
        }

        match probe_type.as_str() {
            "liveness" => Ok(Self::Liveness),
            "basic" => {
                if parts.len() < 2 {
                    return Err(Error::InvalidProxyProber { prober: s.to_owned() });
                }
                let destination_address = HostAddress::from_str(parts[1])?;
                Ok(Self::Basic { destination_address })
            }
            _ => Err(Error::InvalidProxyProber { prober: s.to_owned() }),
        }
    }
}

impl TryInto<Prober> for ProberConfig {
    type Error = Error;

    fn try_into(self) -> Result<Prober, Self::Error> {
        macro_rules! try_parse_url {
            ($url:ident) => {
                match Url::parse($url.as_str()) {
                    Ok(url) => url,
                    Err(source) => return Err(Error::ParseUrl { source, url: $url.to_owned() }),
                }
            };
        }

        match self {
            Self::Liveness => Ok(LivenessProber.into()),
            Self::Basic { destination_address } => Ok(BasicProber::new(destination_address).into()),
            Self::HttpGet { url, expected_response_code } => {
                Ok(HttpProber::get(try_parse_url!(url), expected_response_code).into())
            }
            Self::HttpHead { url, expected_response_code } => {
                Ok(HttpProber::head(try_parse_url!(url), expected_response_code).into())
            }
            Self::HttpDelete { url, expected_response_code } => {
                Ok(HttpProber::delete(try_parse_url!(url), expected_response_code).into())
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyServerFile {
    proxy_servers: Vec<ProxyHost>,
}

impl ProxyServerFile {
    pub fn from_text(text: &str) -> Result<Self, Error> {
        let proxy_servers =
            text.lines().map(str::trim).filter_map(|line| ProxyHost::from_str(line).ok()).collect();
        Ok(Self { proxy_servers })
    }

    pub fn from_json(json: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(json).context(error::ParseProxyServerJsonSnafu)
    }

    pub fn from_toml(toml: &[u8]) -> Result<Self, Error> {
        let content = String::from_utf8_lossy(toml);
        toml::from_str(content.to_string().as_str()).context(error::ParseProxyServerTomlSnafu)
    }

    pub fn load<P: AsRef<Path>>(file_path: P) -> Result<Self, Error> {
        let file_path = file_path.as_ref();
        match file_path.extension() {
            None => Err(Error::DetectProxyChainFormat { file_path: file_path.to_owned() }),
            Some(ext) => match ext.to_str() {
                Some("txt") => Self::load_text_file(file_path),
                Some("json") => Self::load_json_file(file_path),
                Some("toml") => Self::load_toml_file(file_path),
                Some(ext) => Err(Error::ProxyChainFormatNotSupported { format: ext.to_owned() }),
                None => Err(Error::DetectProxyChainFormat { file_path: file_path.to_owned() }),
            },
        }
    }

    pub fn load_text_file<P: AsRef<Path>>(file_path: P) -> Result<Self, Error> {
        let content =
            std::fs::read_to_string(&file_path).context(error::LoadProxyServerFileSnafu)?;
        Self::from_text(&content)
    }

    pub fn load_json_file<P: AsRef<Path>>(file_path: P) -> Result<Self, Error> {
        let content = std::fs::read(&file_path).context(error::LoadProxyServerFileSnafu)?;
        Self::from_json(&content)
    }

    pub fn load_toml_file<P: AsRef<Path>>(file_path: P) -> Result<Self, Error> {
        let content = std::fs::read(&file_path).context(error::LoadProxyServerFileSnafu)?;
        Self::from_toml(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_server_file_from_text() {
        let text = r#"
socks4://50.235.92.65:32100
socks5://96.69.174.252:39593
socks4a://67.204.1.222:64312
   http://50.233.42.98:30717
http://52.2.42.8

http://70.83.106.82:55801

socks4a://45.5.94.34:56731
socks5://50.30.24.217:54321
"#;

        use ProxyHost::{HttpTunnel, Socks4a, Socks5};
        let file = ProxyServerFile {
            proxy_servers: vec![
                Socks4a { host: "50.235.92.65".to_owned(), port: 32100, id: None },
                Socks5 {
                    host: "96.69.174.252".to_owned(),
                    port: 39593,
                    username: None,
                    password: None,
                },
                Socks4a { host: "67.204.1.222".to_owned(), port: 64312, id: None },
                HttpTunnel {
                    host: "50.233.42.98".to_owned(),
                    port: 30717,
                    user_agent: None,
                    username: None,
                    password: None,
                },
                HttpTunnel {
                    host: "52.2.42.8".to_owned(),
                    port: 80,
                    user_agent: None,
                    username: None,
                    password: None,
                },
                HttpTunnel {
                    host: "70.83.106.82".to_owned(),
                    port: 55801,
                    user_agent: None,
                    username: None,
                    password: None,
                },
                Socks4a { host: "45.5.94.34".to_owned(), port: 56731, id: None },
                Socks5 {
                    host: "50.30.24.217".to_owned(),
                    port: 54321,
                    username: None,
                    password: None,
                },
            ],
        };

        assert_eq!(ProxyServerFile::from_text(text).unwrap(), file);
    }

    #[test]
    fn proxy_server_file_from_json() {
        let json = r#"
{
  "proxyServers": [
    { "type": "socks5", "host": "127.99.0.1", "port": 3128 },
    { "type": "socks4a", "host": "127.99.0.2", "port": 3128 },
    { "type": "httpTunnel", "host": "127.99.0.3", "port": 1080 }
  ]
}
        "#;

        let file = ProxyServerFile {
            proxy_servers: vec![
                ProxyHost::Socks5 {
                    host: "127.99.0.1".to_owned(),
                    port: 3128,
                    username: None,
                    password: None,
                },
                ProxyHost::Socks4a { host: "127.99.0.2".to_owned(), port: 3128, id: None },
                ProxyHost::HttpTunnel {
                    host: "127.99.0.3".to_owned(),
                    port: 1080,
                    username: None,
                    password: None,
                    user_agent: None,
                },
            ],
        };

        assert_eq!(ProxyServerFile::from_json(json.as_bytes()).unwrap(), file);
    }

    #[test]
    fn proxy_server_file_from_toml() {
        let toml = r#"
[[proxyServers]]
type = "socks5"
host = "127.99.0.1"
port = 3128

[[proxyServers]]
type = "socks4a"
host = "127.99.0.2"
port = 3128

[[proxyServers]]
type = "httpTunnel"
host = "127.99.0.3"
port = 1080
        "#;

        let file = ProxyServerFile {
            proxy_servers: vec![
                ProxyHost::Socks5 {
                    host: "127.99.0.1".to_owned(),
                    port: 3128,
                    username: None,
                    password: None,
                },
                ProxyHost::Socks4a { host: "127.99.0.2".to_owned(), port: 3128, id: None },
                ProxyHost::HttpTunnel {
                    host: "127.99.0.3".to_owned(),
                    port: 1080,
                    username: None,
                    password: None,
                    user_agent: None,
                },
            ],
        };

        assert_eq!(ProxyServerFile::from_toml(toml.as_bytes()).unwrap(), file);
    }
}
