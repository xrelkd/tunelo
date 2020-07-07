use std::{collections::HashSet, convert::TryFrom};

use tokio::io::AsyncRead;

use crate::{
    authentication::AuthenticationMethod,
    protocol::socks::{consts, Address, AddressType, Error, SocksCommand, SocksVersion},
};

mod datagram;

pub use self::datagram::Datagram;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[allow(dead_code)]
pub enum Method {
    /// No Authentication
    NoAuthentication,

    /// GSSAPI is gssapi method
    GSSAPI, // MUST support // todo

    /// UsernamePassword is username/assword auth method
    UsernamePassword, // SHOULD support

    /// Not acceptable authentication method
    NotAcceptable,
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::NoAuthentication => write!(f, "No Authentication"),
            Method::GSSAPI => write!(f, "GSSAPI"),
            Method::UsernamePassword => write!(f, "Username/password authentication method"),
            Method::NotAcceptable => write!(f, "Not acceptable authentication method"),
        }
    }
}

impl From<AuthenticationMethod> for Method {
    fn from(method: AuthenticationMethod) -> Method {
        match method {
            AuthenticationMethod::NoAuthentication => Method::NoAuthentication,
            AuthenticationMethod::UsernamePassword => Method::UsernamePassword,
        }
    }
}

impl Into<u8> for Method {
    fn into(self) -> u8 {
        match self {
            Method::NoAuthentication => consts::SOCKS5_AUTH_METHOD_NONE,
            Method::GSSAPI => consts::SOCKS5_AUTH_METHOD_GSSAPI,
            Method::UsernamePassword => consts::SOCKS5_AUTH_METHOD_PASSWORD,
            Method::NotAcceptable => consts::SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE,
        }
    }
}

impl From<u8> for Method {
    fn from(method: u8) -> Method {
        match method {
            consts::SOCKS5_AUTH_METHOD_NONE => Method::NoAuthentication,
            consts::SOCKS5_AUTH_METHOD_GSSAPI => Method::GSSAPI,
            consts::SOCKS5_AUTH_METHOD_PASSWORD => Method::UsernamePassword,
            _ => Method::NotAcceptable,
        }
    }
}

impl Method {
    #[inline]
    pub fn serialized_len() -> usize { std::mem::size_of::<u8>() }
}

#[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
pub enum Command {
    TcpConnect,
    TcpBind,
    UdpAssociate,
}

impl From<SocksCommand> for Command {
    fn from(cmd: SocksCommand) -> Command {
        match cmd {
            SocksCommand::TcpConnect => Command::TcpConnect,
            SocksCommand::TcpBind => Command::TcpBind,
            SocksCommand::UdpAssociate => Command::UdpAssociate,
        }
    }
}

impl TryFrom<u8> for Command {
    type Error = Error;

    fn try_from(cmd: u8) -> Result<Command, Error> {
        match cmd {
            consts::SOCKS5_CMD_TCP_CONNECT => Ok(Command::TcpConnect),
            consts::SOCKS5_CMD_TCP_BIND => Ok(Command::TcpBind),
            consts::SOCKS5_CMD_UDP_ASSOCIATE => Ok(Command::UdpAssociate),
            command => Err(Error::InvalidCommand { command }),
        }
    }
}

impl Into<u8> for Command {
    fn into(self) -> u8 {
        match self {
            Command::TcpConnect => consts::SOCKS5_CMD_TCP_CONNECT,
            Command::TcpBind => consts::SOCKS5_CMD_TCP_BIND,
            Command::UdpAssociate => consts::SOCKS5_CMD_UDP_ASSOCIATE,
        }
    }
}

impl Command {
    #[inline]
    pub fn serialized_len() -> usize { std::mem::size_of::<u8>() }
}

//  +----+----------+----------+
//  |VER | NMETHODS | METHODS  |
//  +----+----------+----------+
//  | 1  |    1     | 1 to 255 |
//  +----+----------+----------+
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandshakeRequest {
    methods: HashSet<Method>,
}

impl HandshakeRequest {
    pub fn new(methods: Vec<Method>) -> HandshakeRequest {
        let methods = methods.into_iter().fold(HashSet::new(), |mut methods, method| {
            methods.insert(method);
            methods
        });
        HandshakeRequest { methods }
    }

    pub async fn from_reader<R>(client: &mut R) -> Result<HandshakeRequest, Error>
    where
        R: AsyncRead + Unpin,
    {
        use tokio::io::AsyncReadExt;
        let nmethods = client.read_u8().await.map_err(|source| Error::ReadStream { source })?;
        if nmethods == 0 {
            return Err(Error::BadRequest);
        }

        let mut buf = vec![0u8; nmethods as usize];
        client.read_exact(&mut buf).await.map_err(|source| Error::ReadStream { source })?;

        let methods = buf.into_iter().map(Method::from).collect();
        debug!("Got NegotiationRequest: {:?} {} {:?}", SocksVersion::V5, nmethods, methods);

        Ok(HandshakeRequest { methods })
    }

    pub fn contains_method(&self, method: Method) -> bool { self.methods.contains(&method) }

    #[allow(dead_code)]
    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut methods_vec = self.methods.iter().cloned().map(Into::into).collect::<Vec<u8>>();
        methods_vec.sort();
        let nmethods = methods_vec.len() as u8;

        let mut buf = Vec::with_capacity(self.serialized_len());

        buf.push(SocksVersion::V5.into());
        buf.push(nmethods);
        buf.extend(methods_vec);
        buf
    }

    #[inline]
    pub fn serialized_len(&self) -> usize {
        SocksVersion::serialized_len() + std::mem::size_of::<u8>() + self.methods.len()
    }
}

// +----+--------+
// |VER | METHOD |
// +----+--------+
// | 1  |   1    |
// +----+--------+

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct HandshakeReply {
    pub method: Method,
}

impl HandshakeReply {
    pub fn new(method: Method) -> HandshakeReply { HandshakeReply { method } }

    pub async fn from_reader<R>(rdr: &mut R) -> Result<HandshakeReply, Error>
    where
        R: AsyncRead + Unpin,
    {
        use tokio::io::AsyncReadExt;

        let mut buf = [0u8; 2];
        rdr.read_exact(&mut buf).await.map_err(|source| Error::ReadStream { source })?;

        match SocksVersion::try_from(buf[0]) {
            Ok(SocksVersion::V4) => return Err(Error::BadReply),
            Ok(SocksVersion::V5) => {}
            Err(err) => return Err(err),
        }

        let method = Method::from(buf[1]);
        Ok(HandshakeReply { method })
    }

    #[inline]
    pub fn serialized_len() -> usize { SocksVersion::serialized_len() + Method::serialized_len() }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> { vec![SocksVersion::V5.into(), self.method.into()] }

    #[inline]
    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }
}

// UserPassNegotiationRequest is the negotiation username/password reqeust
// packet
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UserPasswordHandshakeRequest {
    pub version: UserPasswordVersion,
    pub user_name: Vec<u8>,
    pub password: Vec<u8>,
}

impl UserPasswordHandshakeRequest {
    #[inline]
    pub fn serialized_len(&self) -> usize {
        SocksVersion::serialized_len() + self.user_name.len() + self.password.len()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.serialized_len());
        buf.push(self.version.into());
        buf.extend(&self.user_name);
        buf.extend(&self.password);
        buf
    }

    pub fn into_bytes(&self) -> Vec<u8> { self.to_bytes() }

    pub async fn from_reader<R>(client: &mut R) -> Result<UserPasswordHandshakeRequest, Error>
    where
        R: AsyncRead + Unpin,
    {
        use tokio::io::AsyncReadExt;

        let mut buf = [0u8; 2];
        client.read_exact(&mut buf).await.map_err(|source| Error::ReadStream { source })?;

        let user_len = buf[1] as usize;
        if user_len == 0 {
            return Err(Error::BadRequest);
        }

        let version = UserPasswordVersion::try_from(buf[0])?;
        if version != UserPasswordVersion::V1 {
            return Err(Error::InvalidUserPasswordVersion { version: buf[0] });
        }

        let mut user_name = vec![0u8; user_len];
        client.read_exact(&mut user_name).await.map_err(|source| Error::ReadStream { source })?;

        let password = {
            let password_len =
                client.read_u8().await.map_err(|source| Error::ReadStream { source })? as usize;
            if password_len == 0 {
                return Err(Error::BadRequest);
            }

            let mut password = vec![0u8; password_len];
            client
                .read_exact(&mut password)
                .await
                .map_err(|source| Error::ReadStream { source })?;
            password
        };

        Ok(UserPasswordHandshakeRequest { version: UserPasswordVersion::V1, user_name, password })
    }
}

// UserPasswordHandshakeReply is the username/password reply packet
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct UserPasswordHandshakeReply {
    pub version: UserPasswordVersion,
    pub status: UserPasswordStatus,
}

impl UserPasswordHandshakeReply {
    pub async fn from_reader<R>(reader: &mut R) -> Result<UserPasswordHandshakeReply, Error>
    where
        R: AsyncRead + Unpin,
    {
        use tokio::io::AsyncReadExt;

        let mut buf = [0u8; 2];
        reader.read(&mut buf).await.map_err(|source| Error::ReadStream { source })?;
        let version = UserPasswordVersion::try_from(buf[0])?;
        let status = UserPasswordStatus::from(buf[1]);
        Ok(UserPasswordHandshakeReply { version, status })
    }

    pub fn success() -> UserPasswordHandshakeReply {
        UserPasswordHandshakeReply {
            version: UserPasswordVersion::V1,
            status: UserPasswordStatus::Success,
        }
    }

    pub fn failure() -> UserPasswordHandshakeReply {
        UserPasswordHandshakeReply {
            version: UserPasswordVersion::V1,
            status: UserPasswordStatus::Failure,
        }
    }

    #[inline]
    pub fn serialized_len() -> usize {
        SocksVersion::serialized_len() + UserPasswordStatus::serialized_len()
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> { vec![self.version.into(), self.status.into()] }

    #[inline]
    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }
}

// Request is the request packet
#[derive(Debug)]
pub struct Request {
    pub command: Command,
    pub destination_socket: Address,
}

impl Request {
    pub async fn from_reader<R>(client: &mut R) -> Result<Request, Error>
    where
        R: AsyncRead + Unpin,
    {
        use tokio::io::AsyncReadExt;

        let mut buf = [0u8; 3];
        let _n =
            client.read_exact(&mut buf).await.map_err(|source| Error::ReadStream { source })?;
        let _rsv = buf[2];

        match SocksVersion::try_from(buf[0]) {
            Err(err) => return Err(err),
            Ok(version) if version != SocksVersion::V5 => {
                return Err(Error::UnsupportedSocksVersion { version })
            }
            Ok(_) => {}
        }

        let command = Command::try_from(buf[1])?;
        let destination_socket = Address::from_reader(client).await?;

        let req = Request { command, destination_socket };
        debug!("Got Request: {:?}", req);

        Ok(req)
    }

    #[inline]
    pub fn address_type(&self) -> AddressType { self.destination_socket.address_type() }

    #[inline]
    pub fn serialized_len(&self) -> usize {
        SocksVersion::serialized_len()
            + Command::serialized_len()
            + std::mem::size_of::<u8>()
            + self.destination_socket.serialized_len(SocksVersion::V5)
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let socket_vec = self.destination_socket.to_bytes(SocksVersion::V5);
        let mut buf = Vec::with_capacity(self.serialized_len());
        buf.push(SocksVersion::V5.into());
        buf.push(self.command.into());
        buf.push(0x00);
        buf.extend(socket_vec);
        buf
    }

    #[inline]
    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }
}

// Reply is the reply packet
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Reply {
    pub reply: ReplyField,
    pub bind_socket: Address,
}

impl Reply {
    pub async fn from_reader<R>(reader: &mut R) -> Result<Reply, Error>
    where
        R: AsyncRead + Unpin,
    {
        use tokio::io::AsyncReadExt;

        let mut buf = [0u8; 3];
        let _n =
            reader.read_exact(&mut buf).await.map_err(|source| Error::ReadStream { source })?;
        let _rsv = buf[2];

        match SocksVersion::try_from(buf[0]) {
            Ok(SocksVersion::V4) => return Err(Error::BadReply),
            Ok(SocksVersion::V5) => {}
            Err(err) => return Err(err),
        }

        let reply = ReplyField::from(buf[1]);
        let bind_socket = Address::from_reader(reader).await?;

        Ok(Reply { reply, bind_socket })
    }

    #[inline]
    pub fn serialized_len(&self) -> usize {
        SocksVersion::serialized_len()
            + ReplyField::serialized_len()
            + std::mem::size_of::<u8>()
            + self.bind_socket.serialized_len(SocksVersion::V5)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let socket_vec = self.bind_socket.to_bytes(SocksVersion::V5);
        let mut buf = Vec::with_capacity(self.serialized_len());
        buf.push(SocksVersion::V5.into());
        buf.push(self.reply.into());
        buf.push(0x00);
        buf.extend(socket_vec);
        buf
    }

    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }

    pub fn success(bind_socket: Address) -> Reply {
        Reply { reply: ReplyField::Success, bind_socket }
    }

    pub fn unreachable(address_type: AddressType) -> Reply {
        Reply { reply: ReplyField::HostUnreachable, bind_socket: Self::empty_socket(address_type) }
    }

    pub fn not_supported(address_type: AddressType) -> Reply {
        Reply {
            reply: ReplyField::CommandNotSupported,
            bind_socket: Self::empty_socket(address_type),
        }
    }

    #[inline]
    fn empty_socket(address_type: AddressType) -> Address {
        match address_type {
            AddressType::Ipv4 | AddressType::Domain => Address::empty_ipv4(),
            AddressType::Ipv6 => Address::empty_ipv6(),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub enum ReplyField {
    // RepSuccess means that success for repling
    Success,

    // RepServerFailure means the server failure
    ServerFailure,

    // RepNotAllowed means the request not allowed
    NotAllowed,

    // RepNetworkUnreachable means the network unreachable
    NetworkUnreachable,

    // RepHostUnreachable means the host unreachable
    HostUnreachable,

    // RepConnectionRefused means the connection refused
    ConnectionRefused,

    // RepTTLExpired means the TTL expired
    TTLExpired,

    // RepCommandNotSupported means the request command not supported
    CommandNotSupported,

    // RepAddressNotSupported means the request address not supported
    AddressNotSupported,

    Unknown,
}

impl Into<u8> for ReplyField {
    fn into(self) -> u8 {
        match self {
            ReplyField::Success => consts::SOCKS5_REPLY_SUCCEEDED,
            ReplyField::ServerFailure => consts::SOCKS5_REPLY_GENERAL_FAILURE,
            ReplyField::NotAllowed => consts::SOCKS5_REPLY_CONNECTION_NOT_ALLOWED,
            ReplyField::NetworkUnreachable => consts::SOCKS5_REPLY_NETWORK_UNREACHABLE,
            ReplyField::HostUnreachable => consts::SOCKS5_REPLY_HOST_UNREACHABLE,
            ReplyField::ConnectionRefused => consts::SOCKS5_REPLY_CONNECTION_REFUSED,
            ReplyField::TTLExpired => consts::SOCKS5_REPLY_TTL_EXPIRED,
            ReplyField::CommandNotSupported => consts::SOCKS5_REPLY_COMMAND_NOT_SUPPORTED,
            ReplyField::AddressNotSupported => consts::SOCKS5_REPLY_ADDRESS_TYPE_NOT_SUPPORTED,
            ReplyField::Unknown => consts::SOCKS5_REPLY_UNKNOWN,
        }
    }
}

impl From<u8> for ReplyField {
    fn from(v: u8) -> ReplyField {
        match v {
            consts::SOCKS5_REPLY_SUCCEEDED => ReplyField::Success,
            consts::SOCKS5_REPLY_GENERAL_FAILURE => ReplyField::ServerFailure,
            consts::SOCKS5_REPLY_CONNECTION_NOT_ALLOWED => ReplyField::NotAllowed,
            consts::SOCKS5_REPLY_NETWORK_UNREACHABLE => ReplyField::NetworkUnreachable,
            consts::SOCKS5_REPLY_HOST_UNREACHABLE => ReplyField::HostUnreachable,
            consts::SOCKS5_REPLY_CONNECTION_REFUSED => ReplyField::ConnectionRefused,
            consts::SOCKS5_REPLY_TTL_EXPIRED => ReplyField::TTLExpired,
            consts::SOCKS5_REPLY_COMMAND_NOT_SUPPORTED => ReplyField::CommandNotSupported,
            consts::SOCKS5_REPLY_ADDRESS_TYPE_NOT_SUPPORTED => ReplyField::AddressNotSupported,
            _ => ReplyField::Unknown,
        }
    }
}

impl ReplyField {
    #[inline]
    pub fn serialized_len() -> usize { std::mem::size_of::<u8>() }
}

#[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
pub enum UserPasswordVersion {
    V1,
}

impl UserPasswordVersion {
    #[inline]
    pub fn serialized_len() -> usize { std::mem::size_of::<u8>() }
}

impl TryFrom<u8> for UserPasswordVersion {
    type Error = Error;

    fn try_from(cmd: u8) -> Result<UserPasswordVersion, Error> {
        match cmd {
            0x01 => Ok(UserPasswordVersion::V1),
            version => Err(Error::InvalidUserPasswordVersion { version }),
        }
    }
}

impl Into<u8> for UserPasswordVersion {
    fn into(self) -> u8 {
        match self {
            UserPasswordVersion::V1 => 0x01,
        }
    }
}

#[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
pub enum UserPasswordStatus {
    Success,
    Failure,
}

impl From<u8> for UserPasswordStatus {
    fn from(cmd: u8) -> UserPasswordStatus {
        match cmd {
            0x00 => UserPasswordStatus::Success,
            _n => UserPasswordStatus::Failure,
        }
    }
}

impl Into<u8> for UserPasswordStatus {
    fn into(self) -> u8 {
        match self {
            UserPasswordStatus::Success => 0x00,
            UserPasswordStatus::Failure => 0x01,
        }
    }
}

impl UserPasswordStatus {
    #[inline]
    pub fn serialized_len() -> usize { std::mem::size_of::<u8>() }
}

#[cfg(test)]
mod tests {}
