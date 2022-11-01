use crate::alloc::string::ToString;
use crate::compat::borrow::Cow;
use crate::compat::string::String;

use core::fmt::{self, Display, Formatter};
use core::ops::Deref;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// A new type around `Cow<'_, str>` that borrows from input.
///
/// Contrary to `Cow<_, str>` the `Decode` impl for this type will always borrow
/// from input so using it in types like `Option`, `Vec<_>` etc will not produce
/// owned element values.
#[derive(
    Debug, Clone, Encode, Decode, Serialize, Deserialize, Eq, PartialOrd, Ord, Hash, Default,
)]
#[cbor(transparent)]
#[serde(transparent)]
pub struct CowStr<'a>(
    #[b(0)]
    #[serde(borrow)]
    pub Cow<'a, str>,
);

impl CowStr<'_> {
    /// Returns true if the data is borrowed, i.e. if to_mut would require additional work.
    pub fn is_borrowed(&self) -> bool {
        matches!(self.0, Cow::Borrowed(_))
    }

    /// Create and return owned CowStr
    pub fn to_owned<'r>(&self) -> CowStr<'r> {
        CowStr(Cow::Owned(self.0.to_string()))
    }

    /// Turn into owned CowStr
    pub fn into_owned(self) -> String {
        self.0.into_owned()
    }
}

impl<'a> From<&'a str> for CowStr<'a> {
    fn from(s: &'a str) -> Self {
        CowStr(Cow::Borrowed(s))
    }
}

impl<'a> From<&'a String> for CowStr<'a> {
    fn from(s: &'a String) -> Self {
        CowStr(Cow::Borrowed(s.as_str()))
    }
}

impl From<String> for CowStr<'_> {
    fn from(s: String) -> Self {
        CowStr(Cow::Owned(s))
    }
}

impl<'a> From<CowStr<'a>> for Cow<'a, str> {
    fn from(c: CowStr<'a>) -> Self {
        c.0
    }
}

impl<'a> Deref for CowStr<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Display for CowStr<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a, S: ?Sized + AsRef<str>> PartialEq<S> for CowStr<'a> {
    fn eq(&self, other: &S) -> bool {
        self.0 == other.as_ref()
    }
}

impl<'a> AsRef<str> for CowStr<'a> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
