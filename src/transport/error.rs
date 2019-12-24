use crate::client;

#[derive(Debug)]
pub enum Error {
    StdIo(std::io::Error),
    ProxyClient(client::Error),
    NameResolver,
    FailedToInitializeNameResolver(String),
    FailedToResolveDomainName,
    ConnectForbiddenHost,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::StdIo(err)
    }
}

impl From<client::Error> for Error {
    fn from(err: client::Error) -> Error {
        Error::ProxyClient(err)
    }
}
