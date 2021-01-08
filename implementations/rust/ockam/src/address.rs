use alloc::fmt;
use alloc::string::{String, ToString};
use core::fmt::{Display, Formatter};
use core::hash::{Hash, Hasher};

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum AddressType {
    Worker = 0,
    Undefined = 255,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Address {
    address_type: AddressType,
    inner: String,
}

impl Address {
    pub fn new<T: ToString>(s: T) -> Address {
        Address::for_type(AddressType::Undefined, s)
    }

    pub fn for_type<T: ToString>(address_type: AddressType, s: T) -> Address {
        Address {
            address_type,
            inner: s.to_string(),
        }
    }

    pub fn for_worker<T: ToString>(s: T) -> Address {
        Address::for_type(AddressType::Worker, s)
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl From<String> for Address {
    fn from(s: String) -> Self {
        Address::new(s.clone())
    }
}

impl From<&str> for Address {
    fn from(s: &str) -> Self {
        Address::from(String::from(s))
    }
}

impl Into<String> for Address {
    fn into(self) -> String {
        self.inner.clone()
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
            address: Address,
        }

        impl Addressable for Thing {
            fn address(&self) -> Address {
                return self.address.clone();
            }
        }

        let test = "test".to_string();

        let thing = Thing {
            address: Address::for_worker(test.clone()),
        };
        assert_eq!(Address::for_worker("test"), thing.address());

        let addr: String = thing.address().into();
        assert_eq!(test, addr);

        let mut map = hashbrown::HashMap::new();
        map.insert(thing.address(), true);

        assert!(map.get(&thing.address()).unwrap());

        assert_eq!("test", format!("{}", thing.address()))
    }
}
