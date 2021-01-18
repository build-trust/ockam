use alloc::fmt;
use alloc::string::{String, ToString};
use core::fmt::{Display, Formatter};
use core::hash::{Hash, Hasher};

#[derive(Clone, Debug, Eq)]
pub struct Address {
    inner: String,
}

impl Address {
    pub fn new<T: ToString>(s: T) -> Address {
        Address {
            inner: s.to_string(),
        }
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl From<String> for Address {
    fn from(s: String) -> Self {
        Address::new(s)
    }
}

impl From<&str> for Address {
    fn from(s: &str) -> Self {
        Address::from(String::from(s))
    }
}

impl Into<String> for Address {
    fn into(self) -> String {
        self.inner
    }
}

impl Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

pub trait Addressable {
    fn address(&self) -> Address;
}

#[cfg(test)]
mod test {
    use crate::address::{Address, Addressable};
    use alloc::string::{String, ToString};

    #[test]
    pub fn test_addressable() {
        struct Thing {
            address: String,
        }
        impl Addressable for Thing {
            fn address(&self) -> Address {
                return self.address.clone().into();
            }
        }

        let test = "test".to_string();

        let thing = Thing {
            address: test.clone(),
        };
        assert_eq!(Address::from("test"), thing.address());

        let addr: String = thing.address().into();
        assert_eq!(test, addr);

        let mut map = hashbrown::HashMap::new();
        map.insert(thing.address(), true);

        assert!(map.get(&thing.address()).unwrap());
    }
}
