use self::SocketAddress::*;
use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming as AddrIncomingTCP, AddrStream};
use hyper::server::Builder as HyperBuilder;
use hyper::server::Server as HyperServer;
#[cfg(unix)]
use hyperlocal::SocketIncoming;
use std::error::Error;
use std::io;
use std::net::{Ipv4Addr, SocketAddrV4, SocketAddrV6};
#[cfg(unix)]
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
#[cfg(unix)]
use tokio::net::UnixStream;

#[derive(Clone, Debug)]
pub(crate) enum SocketAddress {
    IPV4(SocketAddrV4),
    IPV6(SocketAddrV6),
    #[cfg(unix)]
    UnixDomainSocket(PathBuf),
}

fn hyper_to_io(err: hyper::Error) -> io::Error {
    *err.into_cause()
        .expect("unexpected error: non-io hyper error")
        .downcast::<io::Error>()
        .expect("unexpected error: non-io hyper error")
}

impl SocketAddress {
    pub(crate) fn bind(self) -> io::Result<Incoming> {
        match self {
            IPV4(addr) => AddrIncomingTCP::bind(&addr.into())
                .map_err(hyper_to_io)
                .map(Incoming::TCP),
            IPV6(addr) => AddrIncomingTCP::bind(&addr.into())
                .map_err(hyper_to_io)
                .map(Incoming::TCP),
            #[cfg(unix)]
            UnixDomainSocket(path) => SocketIncoming::bind(path).map(Incoming::UnixDomainSocket),
        }
    }

    pub(crate) fn bind_hyper(self) -> io::Result<HyperBuilder<Incoming>> {
        self.bind().map(HyperServer::builder)
    }
}

impl FromStr for SocketAddress {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn from_str_impl(s: &str) -> Option<SocketAddress> {
            match s.chars().next()? {
                // ipv6 addr: /\[[0-9a-fA-F:]+\]/
                '[' => SocketAddrV6::from_str(s).ok().map(IPV6),
                // ipv4 addr: /[0-9]+(\.[0-9]+){3}/
                '0'..='9' => SocketAddrV4::from_str(s).ok().map(IPV4),
                // ipv4 port only
                ':' => s
                    .split_at(1)
                    .1
                    .parse::<u16>()
                    .ok()
                    .map(|port| SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port))
                    .map(IPV4),
                // unix domain socket
                #[cfg(unix)]
                'u' => {
                    // unix: is required
                    if s.len() < 5 {
                        return None;
                    }
                    let (unix_colon, path) = s.split_at(5);
                    if unix_colon != "unix:" {
                        return None;
                    }
                    Some(UnixDomainSocket(PathBuf::from_str(path).ok()?))
                }
                _ => None,
            }
        }
        return from_str_impl(s).ok_or(AddrParseError(()));
    }
}

impl std::fmt::Display for SocketAddress {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IPV4(addr) => addr.fmt(fmt),
            IPV6(addr) => addr.fmt(fmt),
            #[cfg(unix)]
            UnixDomainSocket(path) => write!(fmt, "unix:{}", path.display()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddrParseError(());

impl std::fmt::Display for AddrParseError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("invalid Socket address syntax")
    }
}

impl Error for AddrParseError {}

include!("socket_address_macro.rs");

tcp_unix_impl! {
    #[derive(Debug)]
    pub(crate) enum Incoming {
        TCP(AddrIncomingTCP),
        UnixDomainSocket(SocketIncoming),
    }

    impl Accept {
        type Conn = Connection;
        type Error = io::Error;

        fn poll_accept(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> (Poll<Option<Result<Connection, io::Error>>>) {
            map_result
        }
    }
}

tcp_unix_impl! {
    #[derive(Debug)]
    pub(crate) enum Connection {
        TCP(AddrStream),
        UnixDomainSocket(UnixStream),
    }

    impl AsyncRead {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> (Poll<io::Result<()>>) {
            simple_return
        }
    }

    impl AsyncWrite {
        fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> (Poll<Result<usize, io::Error>>) {
            simple_return
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> (Poll<Result<(), io::Error>>) {
            simple_return
        }

        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> (Poll<Result<(), io::Error>>) {
            simple_return
        }
    }
}
