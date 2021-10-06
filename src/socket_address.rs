use self::SocketAddress::*;
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddrV4, SocketAddrV6};
#[cfg(unix)]
use std::path::PathBuf;
use std::str::FromStr;

pub(crate) enum SocketAddress {
    IPV4(SocketAddrV4),
    IPV6(SocketAddrV6),
    #[cfg(unix)]
    UnixDomainSocket(PathBuf),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddrParseError(());

impl std::fmt::Display for AddrParseError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("invalid Socket address syntax")
    }
}

impl Error for AddrParseError {}
