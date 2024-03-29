use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Could not bind TcpListener, error: {}", source))]
    BindTcpListener { source: std::io::Error },

    #[snafu(display("Could not accept TCP connection, error: {}", source))]
    AcceptTcpStream { source: std::io::Error },
}
