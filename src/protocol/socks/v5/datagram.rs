use std::{convert::TryFrom, net::SocketAddr};

use bytes::BytesMut;
use snafu::ResultExt;

use crate::{
    common::HostAddress,
    protocol::socks::{error, Address, AddressRef, AddressType, Error, SocksVersion},
};

// Datagram is the UDP packet
#[derive(Clone, Debug)]
pub struct Datagram {
    frag: u8,
    destination_socket: Address,
    data: BytesMut,
}

impl Datagram {
    #[inline]
    #[must_use]
    pub fn new(frag: u8, destination_socket: Address, data: BytesMut) -> Self {
        Self { frag, destination_socket, data }
    }

    pub fn from_bytes(input: &[u8]) -> Result<Self, Error> {
        use std::io::{Cursor, Read};

        use byteorder::{BigEndian, ReadBytesExt};

        let mut input = Cursor::new(input);

        // consume rsv field
        if input.read_u16::<BigEndian>().context(error::ReadStreamSnafu)? != 0x0000 {
            return Err(Error::BadRequest);
        }

        // current fragment number
        let frag = input.read_u8().context(error::ReadStreamSnafu)?;

        let destination_socket =
            match AddressType::try_from(input.read_u8().context(error::ReadStreamSnafu)?)? {
                AddressType::Ipv4 => {
                    let mut host = [0u8; 4];
                    input.read_exact(&mut host).context(error::ReadStreamSnafu)?;

                    let port = input.read_u16::<BigEndian>().context(error::ReadStreamSnafu)?;
                    Address::from(SocketAddr::new(host.into(), port))
                }
                AddressType::Ipv6 => {
                    let mut host = [0u8; 16];
                    input.read_exact(&mut host).context(error::ReadStreamSnafu)?;

                    let port = input.read_u16::<BigEndian>().context(error::ReadStreamSnafu)?;
                    Address::from(SocketAddr::new(host.into(), port))
                }
                AddressType::Domain => {
                    let len = input.read_u8().context(error::ReadStreamSnafu)? as usize;

                    let mut host = vec![0u8; len];
                    input.read_exact(&mut host).context(error::ReadStreamSnafu)?;

                    let port = input.read_u16::<BigEndian>().context(error::ReadStreamSnafu)?;
                    Address::new_domain(&host, port)
                }
            };

        let mut data = BytesMut::new();
        input.read(&mut data[..]).context(error::ReadStreamSnafu)?;
        Ok(Self { frag, destination_socket, data })
    }

    #[inline]
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        let mut buf = self.header_internal(true);
        buf.extend(&self.data);
        buf
    }

    #[inline]
    #[must_use]
    pub fn header(&self) -> Vec<u8> { self.header_internal(false) }

    #[inline]
    #[must_use]
    pub fn data(&self) -> &[u8] { self.data.as_ref() }

    #[inline]
    #[must_use]
    pub const fn frag(&self) -> u8 { self.frag }

    #[inline]
    #[must_use]
    pub fn destination_address(&self) -> &HostAddress { self.destination_socket.as_ref() }

    fn header_internal(&self, extensible: bool) -> Vec<u8> {
        use std::mem::size_of_val;
        let mut dest_sock_vec = self.destination_socket.to_bytes(SocksVersion::V5);

        let mut len = 2 + size_of_val(&self.frag) + dest_sock_vec.len();
        if extensible {
            len += self.data.len()
        }

        let mut buf = Vec::with_capacity(len);
        buf.extend_from_slice(&[0x00, 0x00, self.frag]);
        buf.append(&mut dest_sock_vec);
        buf
    }

    #[inline]
    #[must_use]
    pub fn destruct(self) -> (u8, HostAddress, BytesMut) {
        (self.frag, self.destination_socket.into(), self.data)
    }

    pub fn serialize_header<W: std::io::Write>(
        wrt: &mut W,
        frag: u8,
        destination_socket: &HostAddress,
    ) -> Result<usize, std::io::Error> {
        let dest_sock_vec = AddressRef(destination_socket).to_bytes(SocksVersion::V5);

        let mut n = wrt.write(&[0x00, 0x00, frag])?;
        n += wrt.write(&dest_sock_vec)?;
        Ok(n)
    }

    pub fn serialize<W: std::io::Write>(
        wrt: &mut W,
        frag: u8,
        destination_socket: &HostAddress,
        data: &[u8],
    ) -> Result<usize, std::io::Error> {
        let n = Self::serialize_header(wrt, frag, destination_socket)?;
        Ok(n + wrt.write(data)?)
    }
}
