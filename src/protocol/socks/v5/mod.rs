mod datagram;

use std::{collections::HashSet, convert::TryFrom};

use snafu::ResultExt;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
    authentication::AuthenticationMethod,
    protocol::socks::{consts, error, Address, AddressType, Error, SocksCommand, SocksVersion},
};

pub use self::datagram::Datagram;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
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
            Self::NoAuthentication => write!(f, "No Authentication"),
            Self::GSSAPI => write!(f, "GSSAPI"),
            Self::UsernamePassword => write!(f, "Username/password authentication method"),
            Self::NotAcceptable => write!(f, "Not acceptable authentication method"),
        }
    }
}

impl From<AuthenticationMethod> for Method {
    fn from(method: AuthenticationMethod) -> Self {
        match method {
            AuthenticationMethod::NoAuthentication => Self::NoAuthentication,
            AuthenticationMethod::UsernamePassword => Self::UsernamePassword,
        }
    }
}

impl From<Method> for u8 {
    fn from(val: Method) -> Self {
        match val {
            Method::NoAuthentication => consts::SOCKS5_AUTH_METHOD_NONE,
            Method::GSSAPI => consts::SOCKS5_AUTH_METHOD_GSSAPI,
            Method::UsernamePassword => consts::SOCKS5_AUTH_METHOD_PASSWORD,
            Method::NotAcceptable => consts::SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE,
        }
    }
}

impl From<u8> for Method {
    fn from(method: u8) -> Self {
        match method {
            consts::SOCKS5_AUTH_METHOD_NONE => Self::NoAuthentication,
            consts::SOCKS5_AUTH_METHOD_GSSAPI => Self::GSSAPI,
            consts::SOCKS5_AUTH_METHOD_PASSWORD => Self::UsernamePassword,
            _ => Self::NotAcceptable,
        }
    }
}

impl Method {
    #[inline]
    #[must_use]
    pub const fn serialized_len() -> usize { std::mem::size_of::<u8>() }
}

#[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
pub enum Command {
    TcpConnect,
    TcpBind,
    UdpAssociate,
}

impl From<SocksCommand> for Command {
    fn from(cmd: SocksCommand) -> Self {
        match cmd {
            SocksCommand::TcpConnect => Self::TcpConnect,
            SocksCommand::TcpBind => Self::TcpBind,
            SocksCommand::UdpAssociate => Self::UdpAssociate,
        }
    }
}

impl TryFrom<u8> for Command {
    type Error = Error;

    fn try_from(cmd: u8) -> Result<Self, Error> {
        match cmd {
            consts::SOCKS5_CMD_TCP_CONNECT => Ok(Self::TcpConnect),
            consts::SOCKS5_CMD_TCP_BIND => Ok(Self::TcpBind),
            consts::SOCKS5_CMD_UDP_ASSOCIATE => Ok(Self::UdpAssociate),
            command => Err(Error::InvalidCommand { command }),
        }
    }
}

impl From<Command> for u8 {
    fn from(val: Command) -> Self {
        match val {
            Command::TcpConnect => consts::SOCKS5_CMD_TCP_CONNECT,
            Command::TcpBind => consts::SOCKS5_CMD_TCP_BIND,
            Command::UdpAssociate => consts::SOCKS5_CMD_UDP_ASSOCIATE,
        }
    }
}

impl Command {
    #[inline]
    #[must_use]
    pub const fn serialized_len() -> usize { std::mem::size_of::<u8>() }
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
    #[must_use]
    pub fn new(methods: Vec<Method>) -> Self {
        let methods = methods.into_iter().fold(HashSet::new(), |mut methods, method| {
            methods.insert(method);
            methods
        });
        Self { methods }
    }

    pub async fn from_reader<R>(client: &mut R) -> Result<Self, Error>
    where
        R: AsyncRead + Unpin,
    {
        let nmethods = client.read_u8().await.context(error::ReadStreamSnafu)?;
        if nmethods == 0 {
            return Err(Error::BadRequest);
        }

        let mut buf = vec![0u8; nmethods as usize];
        client.read_exact(&mut buf).await.context(error::ReadStreamSnafu)?;

        let methods = buf.into_iter().map(Method::from).collect();
        tracing::debug!(
            "Got NegotiationRequest: {:?} {} {:?}",
            SocksVersion::V5,
            nmethods,
            methods
        );

        Ok(Self { methods })
    }

    #[must_use]
    pub fn contains_method(&self, method: Method) -> bool { self.methods.contains(&method) }

    #[allow(dead_code)]
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut methods_vec = self.methods.iter().copied().map(Into::into).collect::<Vec<u8>>();
        methods_vec.sort_unstable();
        let nmethods = methods_vec.len() as u8;

        let mut buf = Vec::with_capacity(self.serialized_len());

        buf.push(SocksVersion::V5.into());
        buf.push(nmethods);
        buf.extend(methods_vec);
        buf
    }

    #[inline]
    #[must_use]
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
    #[must_use]
    pub const fn new(method: Method) -> Self { Self { method } }

    pub async fn from_reader<R>(rdr: &mut R) -> Result<Self, Error>
    where
        R: AsyncRead + Unpin,
    {
        let mut buf = [0u8; 2];
        rdr.read_exact(&mut buf).await.context(error::ReadStreamSnafu)?;

        match SocksVersion::try_from(buf[0]) {
            Ok(SocksVersion::V4) => return Err(Error::BadReply),
            Ok(SocksVersion::V5) => {}
            Err(err) => return Err(err),
        }

        let method = Method::from(buf[1]);
        Ok(Self { method })
    }

    #[inline]
    #[must_use]
    pub const fn serialized_len() -> usize {
        SocksVersion::serialized_len() + Method::serialized_len()
    }

    #[inline]
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> { Vec::from([SocksVersion::V5.into(), self.method.into()]) }

    #[inline]
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }
}

// UserPassNegotiationRequest is the negotiation username/password request
// packet
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserPasswordHandshakeRequest {
    pub version: UserPasswordVersion,
    pub user_name: Vec<u8>,
    pub password: Vec<u8>,
}

impl UserPasswordHandshakeRequest {
    #[inline]
    #[must_use]
    pub fn serialized_len(&self) -> usize {
        SocksVersion::serialized_len() + self.user_name.len() + self.password.len()
    }

    #[inline]
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.serialized_len());
        buf.push(self.version.into());
        buf.extend(&self.user_name);
        buf.extend(&self.password);
        buf
    }

    #[inline]
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }

    pub async fn from_reader<R>(client: &mut R) -> Result<Self, Error>
    where
        R: AsyncRead + Unpin,
    {
        let mut buf = [0u8; 2];
        client.read_exact(&mut buf).await.context(error::ReadStreamSnafu)?;

        let user_len = buf[1] as usize;
        if user_len == 0 {
            return Err(Error::BadRequest);
        }

        let version = UserPasswordVersion::try_from(buf[0])?;
        if version != UserPasswordVersion::V1 {
            return Err(Error::InvalidUserPasswordVersion { version: buf[0] });
        }

        let mut user_name = vec![0u8; user_len];
        client.read_exact(&mut user_name).await.context(error::ReadStreamSnafu)?;

        let password = {
            let password_len = client.read_u8().await.context(error::ReadStreamSnafu)? as usize;
            if password_len == 0 {
                return Err(Error::BadRequest);
            }

            let mut password = vec![0u8; password_len];
            client.read_exact(&mut password).await.context(error::ReadStreamSnafu)?;
            password
        };

        Ok(Self { version: UserPasswordVersion::V1, user_name, password })
    }
}

// UserPasswordHandshakeReply is the username/password reply packet
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserPasswordHandshakeReply {
    pub version: UserPasswordVersion,
    pub status: UserPasswordStatus,
}

impl UserPasswordHandshakeReply {
    pub async fn from_reader<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: AsyncRead + Unpin,
    {
        let mut buf = [0u8; 2];
        reader.read(&mut buf).await.context(error::ReadStreamSnafu)?;
        let version = UserPasswordVersion::try_from(buf[0])?;
        let status = UserPasswordStatus::from(buf[1]);
        Ok(Self { version, status })
    }

    #[must_use]
    pub const fn success() -> Self {
        Self { version: UserPasswordVersion::V1, status: UserPasswordStatus::Success }
    }

    #[must_use]
    pub const fn failure() -> Self {
        Self { version: UserPasswordVersion::V1, status: UserPasswordStatus::Failure }
    }

    #[inline]
    #[must_use]
    pub const fn serialized_len() -> usize {
        SocksVersion::serialized_len() + UserPasswordStatus::serialized_len()
    }

    #[inline]
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> { vec![self.version.into(), self.status.into()] }

    #[inline]
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }
}

// Request is the request packet
#[derive(Debug)]
pub struct Request {
    pub command: Command,
    pub destination_socket: Address,
}

impl Request {
    pub async fn from_reader<R>(client: &mut R) -> Result<Self, Error>
    where
        R: AsyncRead + Unpin,
    {
        let mut buf = [0u8; 3];
        let _n = client.read_exact(&mut buf).await.context(error::ReadStreamSnafu)?;
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

        let req = Self { command, destination_socket };
        tracing::debug!("Got Request: {:?}", req);

        Ok(req)
    }

    #[inline]
    #[must_use]
    pub fn address_type(&self) -> AddressType { self.destination_socket.address_type() }

    #[inline]
    #[must_use]
    pub fn serialized_len(&self) -> usize {
        SocksVersion::serialized_len()
            + Command::serialized_len()
            + std::mem::size_of::<u8>()
            + self.destination_socket.serialized_len(SocksVersion::V5)
    }

    #[inline]
    #[must_use]
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
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }
}

// Reply is the reply packet
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Reply {
    pub reply: ReplyField,
    pub bind_socket: Address,
}

impl Reply {
    pub async fn from_reader<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: AsyncRead + Unpin,
    {
        let mut buf = [0u8; 3];
        let _n = reader.read_exact(&mut buf).await.context(error::ReadStreamSnafu)?;
        let _rsv = buf[2];

        match SocksVersion::try_from(buf[0]) {
            Ok(SocksVersion::V4) => return Err(Error::BadReply),
            Ok(SocksVersion::V5) => {}
            Err(err) => return Err(err),
        }

        let reply = ReplyField::from(buf[1]);
        let bind_socket = Address::from_reader(reader).await?;

        Ok(Self { reply, bind_socket })
    }

    #[inline]
    #[must_use]
    pub fn serialized_len(&self) -> usize {
        SocksVersion::serialized_len()
            + ReplyField::serialized_len()
            + std::mem::size_of::<u8>()
            + self.bind_socket.serialized_len(SocksVersion::V5)
    }

    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let socket_vec = self.bind_socket.to_bytes(SocksVersion::V5);
        let mut buf = Vec::with_capacity(self.serialized_len());
        buf.push(SocksVersion::V5.into());
        buf.push(self.reply.into());
        buf.push(0x00);
        buf.extend(socket_vec);
        buf
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }

    #[must_use]
    pub const fn success(bind_socket: Address) -> Self {
        Self { reply: ReplyField::Success, bind_socket }
    }

    #[must_use]
    pub fn unreachable(address_type: AddressType) -> Self {
        Self { reply: ReplyField::HostUnreachable, bind_socket: Self::empty_socket(address_type) }
    }

    #[must_use]
    pub fn not_supported(address_type: AddressType) -> Self {
        Self {
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
    // RepSuccess means that success for replying
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

impl From<ReplyField> for u8 {
    fn from(val: ReplyField) -> Self {
        match val {
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
    fn from(v: u8) -> Self {
        match v {
            consts::SOCKS5_REPLY_SUCCEEDED => Self::Success,
            consts::SOCKS5_REPLY_GENERAL_FAILURE => Self::ServerFailure,
            consts::SOCKS5_REPLY_CONNECTION_NOT_ALLOWED => Self::NotAllowed,
            consts::SOCKS5_REPLY_NETWORK_UNREACHABLE => Self::NetworkUnreachable,
            consts::SOCKS5_REPLY_HOST_UNREACHABLE => Self::HostUnreachable,
            consts::SOCKS5_REPLY_CONNECTION_REFUSED => Self::ConnectionRefused,
            consts::SOCKS5_REPLY_TTL_EXPIRED => Self::TTLExpired,
            consts::SOCKS5_REPLY_COMMAND_NOT_SUPPORTED => Self::CommandNotSupported,
            consts::SOCKS5_REPLY_ADDRESS_TYPE_NOT_SUPPORTED => Self::AddressNotSupported,
            _ => Self::Unknown,
        }
    }
}

impl ReplyField {
    #[inline]
    #[must_use]
    pub const fn serialized_len() -> usize { std::mem::size_of::<u8>() }
}

#[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
pub enum UserPasswordVersion {
    V1,
}

impl UserPasswordVersion {
    #[inline]
    #[must_use]
    pub const fn serialized_len() -> usize { std::mem::size_of::<u8>() }
}

impl TryFrom<u8> for UserPasswordVersion {
    type Error = Error;

    fn try_from(cmd: u8) -> Result<Self, Error> {
        match cmd {
            0x01 => Ok(Self::V1),
            version => Err(Error::InvalidUserPasswordVersion { version }),
        }
    }
}

impl From<UserPasswordVersion> for u8 {
    fn from(val: UserPasswordVersion) -> Self {
        match val {
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
    fn from(cmd: u8) -> Self {
        match cmd {
            0x00 => Self::Success,
            _n => Self::Failure,
        }
    }
}

impl From<UserPasswordStatus> for u8 {
    fn from(val: UserPasswordStatus) -> Self {
        match val {
            UserPasswordStatus::Success => 0x00,
            UserPasswordStatus::Failure => 0x01,
        }
    }
}

impl UserPasswordStatus {
    #[inline]
    #[must_use]
    pub const fn serialized_len() -> usize { std::mem::size_of::<u8>() }
}

#[cfg(test)]
mod tests {}
