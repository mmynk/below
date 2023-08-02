use std::str;

use nix::errno::Errno;
use nix::libc::ioctl;
use nix::sys::socket::{socket, AddressFamily, SockFlag, SockType};

use crate::errors::Error;

const ETHTOOL_GSSET_INFO: u32 = 0x37;
const ETHTOOL_GSTRINGS: u32 = 0x1b;
const ETHTOOL_GSTATS: u32 = 0x1d;
const ETH_SS_STATS: u32 = 0x1;
const ETH_GSTRING_LEN: usize = 32;

/// Maximum size of an interface name
const IFNAME_MAX_SIZE: usize = 16;

/// MAX_GSTRINGS maximum number of stats entries that ethtool can retrieve
const MAX_GSTRINGS: usize = 8192;

#[derive(Debug)]
#[repr(C)]
struct StringSetInfo {
    cmd: u32,
    reserved: u32,
    mask: u32,
    data: usize,
}

#[derive(Debug)]
#[repr(C)]
struct GStrings {
    pub cmd: u32,
    pub string_set: u32,
    pub len: u32,
    pub data: [u8; MAX_GSTRINGS * ETH_GSTRING_LEN],
}

#[derive(Debug)]
#[repr(C)]
struct GStats {
    pub cmd: u32,
    pub len: u32,
    pub data: [u8; MAX_GSTRINGS * ETH_GSTRING_LEN],
}

#[derive(Debug)]
#[repr(C)]
struct IfReq {
    if_name: [u8; IFNAME_MAX_SIZE],
    if_data: usize,
}

fn _ioctl(fd: i32, if_name: &String, data: usize) -> Result<(), Errno> {
    let mut ifname = [0u8; IFNAME_MAX_SIZE];
    ifname
        .get_mut(..if_name.len())
        .unwrap()
        .copy_from_slice(if_name.as_bytes());
    let mut request = IfReq {
        if_name: ifname,
        if_data: data,
    };

    let exit_code = unsafe { ioctl(fd, nix::libc::SIOCETHTOOL, &mut request) };

    if exit_code != 0 {
        println!("code: {exit_code}, data: {:?}", request.if_data);
        return Err(Errno::from_i32(exit_code));
    }
    Ok(())
}

/// Parses the byte array returned by ioctl for ETHTOOL_GSTRINGS command.
/// In case of error during parsing any stat name,
/// the function returns a `ParseError`.
fn parse_names(
    data: [u8; MAX_GSTRINGS * ETH_GSTRING_LEN],
    length: usize,
) -> Result<Vec<String>, Error> {
    let mut names = Vec::with_capacity(length);
    for i in 0..length {
        match data.get(i * ETH_GSTRING_LEN..(i + 1) * ETH_GSTRING_LEN) {
            Some(name_bytes) => {
                match name_bytes.iter().position(|b| *b == 0) {
                    Some(end) => {
                        match str::from_utf8(&name_bytes[..end]) {
                            Ok(name) => names.push(name.to_string()), // update features vec if all is good
                            Err(err) => return Err(Error::ParseError(err.to_string())),
                        }
                    }
                    None => {
                        return Err(Error::ParseError(String::from(
                            "parse stat name failed while aligning",
                        )))
                    }
                }
            }
            None => {
                return Err(Error::ParseError(format!(
                    "parse stat name failed at offset={}",
                    i
                )))
            }
        }
    }

    Ok(names)
}

/// Parses the byte array returned by ioctl for ETHTOOL_GSTATS command.
/// In case of error during parsing any feature,
/// the function returns a `ParseError`.
fn parse_values(
    data: [u8; MAX_GSTRINGS * ETH_GSTRING_LEN],
    length: usize,
) -> Result<Vec<u64>, Error> {
    let mut values = Vec::with_capacity(length);
    let mut value_bytes = [0u8; 8];
    for i in 0..length {
        let offset = 8 * i;
        match data.get(offset..offset + 8) {
            Some(slice) => {
                value_bytes.copy_from_slice(slice);
                values.push(u64::from_le_bytes(value_bytes));
            }
            None => {
                return Err(Error::ParseError(format!(
                    "parse value failed at offset={}",
                    offset
                )))
            }
        }
    }

    Ok(values)
}

pub struct Ethtool {
    sock_fd: i32,
    if_name: String,
}

impl Ethtool {
    pub fn init(if_name: &str) -> Self {
        let fd = socket(
            AddressFamily::Inet,
            SockType::Datagram,
            SockFlag::empty(),
            None,
        )
        .expect("failed to open socket");

        Self {
            sock_fd: fd,
            if_name: String::from(if_name),
        }
    }

    /// Get the number of stats using ETHTOOL_GSSET_INFO command
    fn gsset_info(&self) -> Result<usize, Errno> {
        let mut sset_info = StringSetInfo {
            cmd: ETHTOOL_GSSET_INFO,
            reserved: 1,
            mask: 1 << ETH_SS_STATS,
            data: 0,
        };

        match _ioctl(
            self.sock_fd,
            &self.if_name,
            &mut sset_info as *mut StringSetInfo as usize,
        ) {
            Ok(_) => Ok(sset_info.data),
            Err(errno) => Err(errno),
        }
    }

    /// Get the feature names using ETHTOOL_GSTRINGS command
    fn gstrings(&self, length: usize) -> Result<Vec<String>, Error> {
        let mut gstrings = GStrings {
            cmd: ETHTOOL_GSTRINGS,
            string_set: ETH_SS_STATS,
            len: length as u32,
            data: [0u8; MAX_GSTRINGS * ETH_GSTRING_LEN],
        };

        match _ioctl(
            self.sock_fd,
            &self.if_name,
            &mut gstrings as *mut GStrings as usize,
        ) {
            Ok(_) => return parse_names(gstrings.data, length),
            Err(errno) => Err(Error::GStringsReadError(errno)),
        }
    }

    /// Get the statistics for the features using EHTOOL_GSTATS command
    fn gstats(&self, features: &Vec<String>) -> Result<Vec<u64>, Error> {
        let length = features.len();
        let mut gstats = GStats {
            cmd: ETHTOOL_GSTATS,
            len: features.len() as u32,
            data: [0u8; MAX_GSTRINGS * ETH_GSTRING_LEN],
        };

        match _ioctl(
            self.sock_fd,
            &self.if_name,
            &mut gstats as *mut GStats as usize,
        ) {
            Ok(_) => return parse_values(gstats.data, length),
            Err(errno) => Err(Error::GStatsReadError(errno)),
        }
    }

    /// Get statistics using ethtool
    /// Equivalent to `ethtool -S <ifname>` command
    pub fn stats(&self) -> Result<Vec<(String, u64)>, Error> {
        let length = match self.gsset_info() {
            Ok(length) => length,
            Err(errno) => {
                return Err(Error::GSSetInfoReadError(errno));
            }
        };

        let features = match self.gstrings(length) {
            Ok(features) => features,
            Err(err) => return Err(err),
        };

        let values = match self.gstats(&features) {
            Ok(values) => values,
            Err(err) => return Err(err),
        };

        let final_stats = features.into_iter().zip(values).collect();
        Ok(final_stats)
    }
}
