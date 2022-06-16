//! In-memory implementation of the [`Abac`] trait.

use super::error::AbacError;
use super::{Abac, Action, Attribute, Conditional, Resource, Subject, Value};
use ockam_core::Result;
use ockam_core::{
    async_trait,
    compat::{collections::BTreeMap, sync::Arc, sync::RwLock},
};

/// `Memory` is an in-memory implementation of the [`Abac`] trait for
/// use by tests and code examples.
#[derive(Debug, Default)]
pub struct Memory {
    /// [`Inner`] implementation of the `Abac` trait
    pub(crate) inner: Arc<RwLock<Inner>>,
}

impl Memory {
    /// Create a new `Memory` ABAC backend
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner::new())),
        }
    }
}

/// `Inner` implements the [`Abac`] trait for the [`Memory`] ABAC backend.
#[derive(Debug, Default)]
pub struct Inner {
    /// subject maps to a set of key-value attributes
    subjects: BTreeMap<Subject, BTreeMap<String, Value>>,
    /// policies map a resource to a set of actions subject to conditions
    policies: BTreeMap<Resource, BTreeMap<Action, Conditional>>,
}

impl Inner {
    /// Implements [`Abac::new`] for [`Memory::inner`].
    pub fn new() -> Self {
        Inner::default()
    }

    /// Implements [`Abac::set_subject`] for [`Memory::inner`].
    pub fn set_subject<I>(&mut self, s: Subject, attrs: I)
    where
        I: IntoIterator<Item = Attribute>,
    {
        self.subjects.insert(s, attrs.into_iter().collect());
    }

    /// Implements [`Abac::set_policy`] for [`Memory::inner`].
    pub fn set_policy(&mut self, r: Resource, a: Action, c: &Conditional) {
        self.policies
            .entry(r)
            .or_insert_with(BTreeMap::new)
            .insert(a, c.clone());
    }

    /// Implements [`Abac::del_subject`] for [`Memory::inner`].
    pub fn del_subject(&mut self, s: &Subject) {
        self.subjects.remove(s);
    }

    /// Implements [`Abac::del_policy`] for [`Memory::inner`].
    pub fn del_policy(&mut self, r: &Resource) {
        self.policies.remove(r);
    }

    /// Implements [`Abac::is_authorized`] for [`Memory::inner`].
    pub fn is_authorized(&self, s: &Subject, r: &Resource, a: &Action) -> bool {
        if let Some(s) = self.subjects.get(s) {
            if let Some(c) = self.policies.get(r).and_then(|p| p.get(a)) {
                return c.apply(s);
            }
        }
        false
    }
}

#[async_trait]
impl Abac for Memory {
    async fn set_subject<I>(&self, s: Subject, attrs: I) -> Result<()>
    where
        I: IntoIterator<Item = Attribute> + Send + 'static,
    {
        match self.inner.write() {
            Ok(mut mem) => {
                mem.set_subject(s, attrs);
                Ok(())
            }
            Err(_) => Err(AbacError::Write.into()),
        }
    }

    async fn set_policy(&self, r: Resource, a: Action, c: &Conditional) -> Result<()> {
        match self.inner.write() {
            Ok(mut mem) => {
                mem.set_policy(r, a, c);
                Ok(())
            }
            Err(_) => Err(AbacError::Write.into()),
        }
    }

    async fn del_subject(&self, s: &Subject) -> Result<()> {
        match self.inner.write() {
            Ok(mut mem) => {
                mem.del_subject(s);
                Ok(())
            }
            Err(_) => Err(AbacError::Write.into()),
        }
    }

    async fn del_policy(&self, r: &Resource) -> Result<()> {
        match self.inner.write() {
            Ok(mut mem) => {
                mem.del_policy(r);
                Ok(())
            }
            Err(_) => Err(AbacError::Write.into()),
        }
    }

    async fn is_authorized(&self, s: &Subject, r: &Resource, a: &Action) -> Result<bool> {
        match self.inner.read() {
            Ok(mem) => Ok(mem.is_authorized(s, r, a)),
            Err(_) => Err(AbacError::Read.into()),
        }
    }
}
