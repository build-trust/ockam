use super::{Buffer, Checked, Code, Codec, Protocol};
use crate::proto::{DnsAddr, Node, Project, Secure, Service, Space, Tcp, Udp, Worker};
use crate::{Error, ProtoValue};
use core::fmt;
use unsigned_varint::decode;

pub struct StdCodec;

impl Codec for StdCodec {
    fn split_str<'a>(
        &self,
        _prefix: &str,
        input: &'a str,
    ) -> Result<(Checked<&'a str>, &'a str), Error> {
        if let Some(p) = input.find('/') {
            let (x, y) = input.split_at(p);
            Ok((Checked(x), y))
        } else {
            Ok((Checked(input), ""))
        }
    }

    fn split_bytes<'a>(
        &self,
        code: Code,
        input: &'a [u8],
    ) -> Result<(Checked<&'a [u8]>, &'a [u8]), Error> {
        match code {
            #[cfg(feature = "std")]
            crate::proto::Ip4::CODE => {
                if input.len() < 4 {
                    return Err(Error::required_bytes(crate::proto::Ip4::CODE, 4));
                }
                let (x, y) = input.split_at(4);
                Ok((Checked(x), y))
            }
            #[cfg(feature = "std")]
            crate::proto::Ip6::CODE => {
                if input.len() < 16 {
                    return Err(Error::required_bytes(crate::proto::Ip6::CODE, 16));
                }
                let (x, y) = input.split_at(16);
                Ok((Checked(x), y))
            }
            Tcp::CODE => {
                if input.len() < 2 {
                    return Err(Error::required_bytes(Tcp::CODE, 2));
                }
                let (x, y) = input.split_at(2);
                Ok((Checked(x), y))
            }
            Udp::CODE => {
                if input.len() < 2 {
                    return Err(Error::required_bytes(Udp::CODE, 2));
                }
                let (x, y) = input.split_at(2);
                Ok((Checked(x), y))
            }
            c @ Worker::CODE
            | c @ DnsAddr::CODE
            | c @ Service::CODE
            | c @ Node::CODE
            | c @ Project::CODE
            | c @ Space::CODE
            | c @ Secure::CODE => {
                let (len, input) = decode::usize(input)?;
                if input.len() < len {
                    return Err(Error::required_bytes(c, len));
                }
                let (x, y) = input.split_at(len);
                Ok((Checked(x), y))
            }
            _ => Err(Error::unregistered(code)),
        }
    }

    fn is_valid_bytes(&self, code: Code, input: Checked<&[u8]>) -> bool {
        match code {
            Worker::CODE => Worker::read_bytes(input).is_ok(),
            #[cfg(feature = "std")]
            crate::proto::Ip4::CODE => crate::proto::Ip4::read_bytes(input).is_ok(),
            #[cfg(feature = "std")]
            crate::proto::Ip6::CODE => crate::proto::Ip6::read_bytes(input).is_ok(),
            Tcp::CODE => Tcp::read_bytes(input).is_ok(),
            Udp::CODE => Udp::read_bytes(input).is_ok(),
            DnsAddr::CODE => DnsAddr::read_bytes(input).is_ok(),
            Service::CODE => Service::read_bytes(input).is_ok(),
            Node::CODE => Node::read_bytes(input).is_ok(),
            Project::CODE => Project::read_bytes(input).is_ok(),
            Space::CODE => Space::read_bytes(input).is_ok(),
            Secure::CODE => Secure::read_bytes(input).is_ok(),
            _ => false,
        }
    }

    fn write_bytes(&self, val: &ProtoValue, buf: &mut dyn Buffer) -> Result<(), Error> {
        match val.code() {
            Worker::CODE => Worker::read_bytes(val.data())?.write_bytes(buf),
            #[cfg(feature = "std")]
            crate::proto::Ip4::CODE => crate::proto::Ip4::read_bytes(val.data())?.write_bytes(buf),
            #[cfg(feature = "std")]
            crate::proto::Ip6::CODE => crate::proto::Ip6::read_bytes(val.data())?.write_bytes(buf),
            Tcp::CODE => Tcp::read_bytes(val.data())?.write_bytes(buf),
            Udp::CODE => Udp::read_bytes(val.data())?.write_bytes(buf),
            DnsAddr::CODE => DnsAddr::read_bytes(val.data())?.write_bytes(buf),
            Service::CODE => Service::read_bytes(val.data())?.write_bytes(buf),
            Node::CODE => Node::read_bytes(val.data())?.write_bytes(buf),
            Project::CODE => Project::read_bytes(val.data())?.write_bytes(buf),
            Space::CODE => Space::read_bytes(val.data())?.write_bytes(buf),
            Secure::CODE => Secure::read_bytes(val.data())?.write_bytes(buf),
            code => return Err(Error::unregistered(code)),
        }
        Ok(())
    }

    fn transcode_str(
        &self,
        prefix: &str,
        value: Checked<&str>,
        buf: &mut dyn Buffer,
    ) -> Result<(), Error> {
        match prefix {
            Worker::PREFIX => {
                Worker::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            #[cfg(feature = "std")]
            crate::proto::Ip4::PREFIX => {
                crate::proto::Ip4::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            #[cfg(feature = "std")]
            crate::proto::Ip6::PREFIX => {
                crate::proto::Ip6::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            Tcp::PREFIX => {
                Tcp::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            Udp::PREFIX => {
                Udp::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            DnsAddr::PREFIX => {
                DnsAddr::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            Service::PREFIX => {
                Service::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            Node::PREFIX => {
                Node::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            Project::PREFIX => {
                Project::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            Space::PREFIX => {
                Space::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            Secure::PREFIX => {
                Secure::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            _ => Err(Error::unregistered_prefix(prefix)),
        }
    }

    fn transcode_bytes(
        &self,
        code: Code,
        value: Checked<&[u8]>,
        f: &mut fmt::Formatter,
    ) -> Result<(), Error> {
        match code {
            Worker::CODE => {
                Worker::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            #[cfg(feature = "std")]
            crate::proto::Ip4::CODE => {
                crate::proto::Ip4::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            #[cfg(feature = "std")]
            crate::proto::Ip6::CODE => {
                crate::proto::Ip6::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            Tcp::CODE => {
                Tcp::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            Udp::CODE => {
                Udp::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            DnsAddr::CODE => {
                DnsAddr::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            Service::CODE => {
                Service::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            Node::CODE => {
                Node::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            Project::CODE => {
                Project::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            Space::CODE => {
                Space::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            Secure::CODE => {
                Secure::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            _ => Err(Error::unregistered(code)),
        }
    }
}
