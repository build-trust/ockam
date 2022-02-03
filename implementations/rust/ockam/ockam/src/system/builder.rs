use crate::SystemHandler;
use ockam_core::{compat::collections::BTreeMap, Address, Message};

struct HandlerData<C, M>
where
    C: Send + 'static,
    M: Message,
{
    inner: Box<dyn SystemHandler<C, M>>,
    routes: BTreeMap<String, Address>,
}

/// An abstraction to build a worker system graph
///
/// When creating a worker system it's important to initialise each
/// handler with the set of internal addresses that it must
/// communicate with.  This structure aims to make initialisation
/// easier.
pub struct SystemBuilder<C, M>
where
    C: Send + 'static,
    M: Message,
{
    /// The set of handlers in this system
    inner: Vec<HandlerData<C, M>>,
}

impl<C, M> SystemBuilder<C, M>
where
    C: Send + 'static,
    M: Message,
{
    /// Create an empty SystemBuilder
    pub fn new() -> Self {
        Self { inner: vec![] }
    }

    /// Add a new handler to this SystemBuilder
    ///
    /// You must at least provide ONE route path (via `.default(addr)`
    /// or `.condition(cond, addr)`), or else initialisation will fail.
    pub fn add<H>(&mut self, handler: H) -> HandlerBuilder<'_, C, M>
    where
        H: SystemHandler<C, M> + 'static,
    {
        HandlerBuilder::new(self, Box::new(handler))
    }
}

/// Builder API for a single SystemHandler
///
/// This type may panic during destruction if not properly initialised
/// first!
pub struct HandlerBuilder<'paren, C, M>
where
    C: Send + 'static,
    M: Message,
{
    routes: Option<BTreeMap<String, Address>>,
    inner: Option<Box<dyn SystemHandler<C, M>>>,
    parent: &'paren mut SystemBuilder<C, M>,
}

impl<'paren, C, M> HandlerBuilder<'paren, C, M>
where
    C: Send + 'static,
    M: Message,
{
    fn new(parent: &'paren mut SystemBuilder<C, M>, inner: Box<dyn SystemHandler<C, M>>) -> Self {
        Self {
            routes: Some(BTreeMap::new()),
            inner: Some(inner),
            parent,
        }
    }

    /// Set an address for the default path
    pub fn default<A: Into<Address>>(mut self, addr: A) -> Self {
        self.routes
            .as_mut()
            .unwrap()
            .insert("default".into(), addr.into());
        self
    }

    /// Attach a conditional forward to this system part
    pub fn condition<S, A>(mut self, label: S, addr: A) -> Self
    where
        S: Into<String>,
        A: Into<Address>,
    {
        let label = label.into();
        assert_ne!(label.as_str(), "default");
        self.routes.as_mut().unwrap().insert(label, addr.into());
        self
    }
}

impl<'paren, C, M> Drop for HandlerBuilder<'paren, C, M>
where
    C: Send + 'static,
    M: Message,
{
    fn drop(&mut self) {
        let inner = core::mem::replace(&mut self.inner, None).unwrap();
        let routes = core::mem::replace(&mut self.routes, None).unwrap();

        assert_ne!(routes.len(), 0);
        self.parent.inner.push(HandlerData { inner, routes })
    }
}
