use std::{
    convert::TryInto,
    path::{Path, PathBuf},
    str::FromStr,
};

use structopt::StructOpt;
use url::Url;

use tunelo::{
    checker::{BasicProber, HttpProber, LivenessProber, Prober, SimpleProxyChecker, TaskReport},
    common::{HostAddress, ProxyHost},
};

use crate::error::Error;

pub async fn run<P: AsRef<Path>>(options: Options, config_file: Option<P>) -> Result<(), Error> {
    let output_path = options.output_path.clone();
    let mut config = match config_file {
        Some(path) => Config::load(path)?.merge(options),
        None => Config::default().merge(options),
    };

    if let Some(file) = config.proxy_server_file {
        let file = ProxyServerFile::load(&file)?;
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

    let mut reports = vec![];
    for checker in checkers {
        let r = checker.run_parallel().await;
        reports.push(r);
    }

    write_reports_to(&mut std::io::stdout(), &reports)
        .map_err(|source| Error::WriteProxyCheckerReport { source })?;

    if let Some(ref path) = &output_path {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&path)
            .map_err(|source| Error::WriteProxyHosts { source })?;

        write_available_proxy_servers(&mut file, &reports)
            .map_err(|source| Error::WriteProxyHosts { source })?;
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
        .filter_map(|r| match r.is_proxy_server_alive() {
            true => Some(r.proxy_server.clone()),
            false => None,
        })
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
            let err = r.error.as_ref().map(|e| e.to_string()).unwrap_or_default();
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

            writeln!(writer, "{}", table)?;
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
                let destination = r.destination.as_ref().map(|d| d.to_string()).unwrap_or_default();
                let err = r.error.as_ref().map(|e| e.to_string()).unwrap_or_default();
                table.add_row(vec![String::new(), destination, destination_reachable, err]);
            }

            writeln!(writer, "{}", table)?;
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
                let method = r.method.as_ref().map(|m| m.to_string()).unwrap_or_default();
                let response_code =
                    r.response_code.as_ref().map(|n| n.to_string()).unwrap_or("N/A".to_owned());
                let url = r.url.as_ref().map(|u| u.to_string()).unwrap_or_default();
                let err = r.error.as_ref().map(|e| e.to_string()).unwrap_or_default();

                table.add_row(vec![String::new(), method, url, response_code, err]);
            }

            writeln!(writer, "{}", table)?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    proxy_servers: Vec<ProxyHost>,
    proxy_server_file: Option<PathBuf>,
    probers: Vec<ProberConfig>,
}

impl Config {
    impl_config_load!(Config);

    fn merge(mut self, opts: Options) -> Config {
        if !opts.proxy_servers.is_empty() {
            self.proxy_servers = opts.proxy_servers;
        }

        if !opts.probers.is_empty() {
            self.probers = opts.probers;
        }

        if opts.proxy_server_file.is_some() {
            self.proxy_server_file = opts.proxy_server_file;
        }

        self
    }
}

impl Default for Config {
    fn default() -> Config {
        let probers = vec![
            ProberConfig::HttpHead {
                url: "https://www.google.com".to_owned(),
                expected_response_code: 200,
            },
            ProberConfig::HttpGet {
                url: "https://ifconfig.me/ip".to_owned(),
                expected_response_code: 200,
            },
        ];
        Config { proxy_servers: vec![], proxy_server_file: None, probers }
    }
}

#[derive(Debug, StructOpt)]
pub struct Options {
    #[structopt(long = "proxy-servers", short = "s", help = "Proxy server list")]
    proxy_servers: Vec<ProxyHost>,

    #[structopt(long = "file", short = "f", help = "Proxy server list file")]
    proxy_server_file: Option<PathBuf>,

    #[structopt(long = "probers", short = "p", help = "Proxy probers")]
    probers: Vec<ProberConfig>,

    #[structopt(long = "output-file", short = "o")]
    output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let parts: Vec<_> = s.split(",").map(|s| s.trim()).collect();

        let probe_type = parts[0].to_lowercase().to_owned();
        if probe_type.starts_with("http") {
            if parts.len() < 3 {
                return Err(Error::InvalidProxyProber { prober: s.to_owned() });
            }

            let url = parts[1].to_owned();
            let expected_response_code =
                parts[2].parse().map_err(|_| Error::InvalidProxyProber { prober: s.to_owned() })?;

            let prober = match probe_type.as_str() {
                "http-get" => ProberConfig::HttpGet { url, expected_response_code },
                "http-head" => ProberConfig::HttpHead { url, expected_response_code },
                "http-delete" => ProberConfig::HttpDelete { url, expected_response_code },
                _ => {
                    return Err(Error::InvalidProxyProber { prober: s.to_owned() });
                }
            };
            return Ok(prober);
        }

        match probe_type.as_str() {
            "liveness" => Ok(ProberConfig::Liveness),
            "basic" => {
                if parts.len() < 2 {
                    return Err(Error::InvalidProxyProber { prober: s.to_owned() });
                }
                let destination_address = HostAddress::from_str(parts[1])?;
                Ok(ProberConfig::Basic { destination_address })
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
            ProberConfig::Liveness => Ok(LivenessProber::new().into()),
            ProberConfig::Basic { destination_address } => {
                Ok(BasicProber::new(destination_address).into())
            }
            ProberConfig::HttpGet { url, expected_response_code } => {
                Ok(HttpProber::get(try_parse_url!(url), expected_response_code).into())
            }
            ProberConfig::HttpHead { url, expected_response_code } => {
                Ok(HttpProber::head(try_parse_url!(url), expected_response_code).into())
            }
            ProberConfig::HttpDelete { url, expected_response_code } => {
                Ok(HttpProber::delete(try_parse_url!(url), expected_response_code).into())
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyServerFile {
    proxy_servers: Vec<ProxyHost>,
}

impl ProxyServerFile {
    pub fn from_json(json: &[u8]) -> Result<ProxyServerFile, Error> {
        serde_json::from_slice(&json).map_err(|source| Error::ParseProxyServerJson { source })
    }

    pub fn from_toml(toml: &[u8]) -> Result<ProxyServerFile, Error> {
        toml::from_slice(&toml).map_err(|source| Error::ParseProxyServerToml { source })
    }

    pub fn load<P: AsRef<Path>>(file_path: P) -> Result<ProxyServerFile, Error> {
        let file_path = file_path.as_ref();
        match file_path.extension() {
            None => return Err(Error::DetectProxyChainFormat { file_path: file_path.to_owned() }),
            Some(ext) => match ext.to_str() {
                Some("json") => ProxyServerFile::load_json_file(file_path),
                Some("toml") => ProxyServerFile::load_toml_file(file_path),
                Some(ext) => Err(Error::ProxyChainFormatNotSupported { format: ext.to_owned() }),
                None => Err(Error::DetectProxyChainFormat { file_path: file_path.to_owned() }),
            },
        }
    }

    pub fn load_json_file<P: AsRef<Path>>(file_path: P) -> Result<ProxyServerFile, Error> {
        let content =
            std::fs::read(&file_path).map_err(|source| Error::LoadProxyServerFile { source })?;
        Self::from_json(&content)
    }

    pub fn load_toml_file<P: AsRef<Path>>(file_path: P) -> Result<ProxyServerFile, Error> {
        let content =
            std::fs::read(&file_path).map_err(|source| Error::LoadProxyServerFile { source })?;
        Self::from_toml(&content)
    }
}

#[cfg(test)]
mod tests {}
