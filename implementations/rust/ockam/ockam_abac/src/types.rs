#[cfg(not(feature = "std"))]
use alloc::string::String;

macro_rules! define {
    ($t:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $t(String);

        impl $t {
            pub fn new<S: Into<String>>(s: S) -> Self {
                Self(s.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $t {
            fn from(s: &str) -> Self {
                Self::new(s)
            }
        }

        impl From<String> for $t {
            fn from(s: String) -> Self {
                Self::new(s)
            }
        }

        impl core::fmt::Display for $t {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

define!(Subject);
define!(Resource);
define!(Action);
