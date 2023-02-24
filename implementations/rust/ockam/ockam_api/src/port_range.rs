use core::str::FromStr;
use ockam_core::compat::fmt::Formatter;
use ockam_core::compat::io::{Error, ErrorKind};

/// Represents a range of port, inclusive start, inclusive end.
/// Always guarantee that end is bigger than start and at least 1 port in the range
#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash)]
pub struct PortRange {
    start: u16,
    end: u16,
}

impl core::fmt::Display for PortRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

impl PortRange {
    pub fn new(start: u16, end: u16) -> Result<Self, Error> {
        if start > end {
            Err(Error::new(
                ErrorKind::InvalidInput,
                "invalid start bigger than end",
            ))
        } else {
            Ok(Self { start, end })
        }
    }

    pub fn start(&self) -> u16 {
        self.start
    }
    pub fn end(&self) -> u16 {
        self.end
    }
}

impl TryFrom<(u16, u16)> for PortRange {
    type Error = Error;

    fn try_from(tuple: (u16, u16)) -> Result<PortRange, Error> {
        Self::new(tuple.0, tuple.1)
    }
}

impl From<PortRange> for (u16, u16) {
    fn from(val: PortRange) -> Self {
        (val.start, val.end)
    }
}

impl FromStr for PortRange {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let tokens: Vec<&str> = text.splitn(2, '-').collect();
        if tokens.len() != 2 {
            return Err(Error::new(ErrorKind::InvalidInput, "invalid port range"));
        }

        let map_err = |cause| Error::new(ErrorKind::InvalidInput, cause);
        #[allow(clippy::get_first)]
        Self::new(
            u16::from_str(tokens.get(0).unwrap()).map_err(map_err)?,
            u16::from_str(tokens.get(1).unwrap()).map_err(map_err)?,
        )
    }
}

impl TryFrom<&str> for PortRange {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::from_str(text)
    }
}

#[test]
fn port_range_parse() -> () {
    assert_eq!(
        PortRange::try_from("10-20").unwrap(),
        PortRange { start: 10, end: 20 }
    );
    assert!(PortRange::try_from("30-10").is_err());
    assert!(PortRange::try_from("10-20-30").is_err());
    assert!(PortRange::try_from("10").is_err());
    assert!(PortRange::try_from("10-").is_err());
    assert!(PortRange::try_from("10,10,30,40").is_err());
}
