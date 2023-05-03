use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::Result;
use anyhow::anyhow;

pub fn alias_parser(arg: &str) -> Result<String> {
    if arg.contains(':') {
        Err(anyhow!("an alias must not contain ':' characters").into())
    } else {
        Ok(arg.to_string())
    }
}

pub(crate) fn socket_addr_parser(input: &str) -> Result<SocketAddr> {
    let to_address_info: Vec<&str> = input.split(':').collect();
    if to_address_info.len() > 2 {
        return Err(anyhow!("Failed to parse to address").into());
    }

    let port: u16 = if to_address_info.len() == 2 {
        to_address_info[1]
            .parse()
            .map_err(|_| anyhow!("Invalid port number"))?
    } else {
        to_address_info[0]
            .parse()
            .map_err(|_| anyhow!("Invalid port number"))?
    };

    let server_ip: Ipv4Addr = if to_address_info.len() < 2 {
        [127, 0, 0, 1].into()
    } else {
        let address_octets: [u8; 4] = {
            let mut octets = [0; 4];
            for (i, octet_str) in to_address_info[0].split('.').enumerate() {
                octets[i] = octet_str
                    .parse()
                    .map_err(|_| anyhow!("Invalid IP address"))?;
            }
            octets
        };
        Ipv4Addr::from(address_octets)
    };

    Ok(SocketAddr::new(IpAddr::V4(server_ip), port))
}
