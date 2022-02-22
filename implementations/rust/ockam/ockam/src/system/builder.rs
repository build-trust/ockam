use crate::{Result, SystemHandler, WorkerSystem};
use ockam_core::{
    compat::{boxed::Box, collections::BTreeMap, string::String},
    Address, Message,
};

struct HandlerData<C, M>
where
    C: Send + 'static,
    M: Message,
{
    addr: Address,
    inner: Box<dyn SystemHandler<C, M> + Send + 'static>,
    routes: BTreeMap<String, Address>,
}

impl<C, M> Clone for HandlerData<C, M>
where
    C: Send + 'static,
    M: Message,
{
    fn clone(&self) -> Self {
        Self {
            inner: dyn_clone::clone_box(&*self.inner),
            addr: self.addr.clone(),
            routes: self.routes.clone(),
        }
    }
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
    inner: BTreeMap<String, HandlerData<C, M>>,
    entry: Option<Address>,
}

impl<C, M> Clone for SystemBuilder<C, M>
where
    C: Send + 'static,
    M: Message,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            entry: self.entry.clone(),
        }
    }
}

impl<C, M> Default for SystemBuilder<C, M>
where
    C: Send + 'static,
    M: Message,
{
    fn default() -> Self {
        Self {
            inner: BTreeMap::new(),
            entry: None,
        }
    }
}

impl<C, M> SystemBuilder<C, M>
where
    C: Send + 'static,
    M: Message,
{
    /// Create an empty SystemBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new handler to this SystemBuilder
    ///
    /// You must at least provide ONE route path (via `.default(addr)`
    /// or `.condition(cond, addr)`), or else initialisation will
    /// fail.
    pub fn add<A, S, H>(&mut self, addr: A, id: S, handler: H) -> HandlerBuilder<'_, C, M>
    where
        A: Into<Address>,
        S: Into<String>,
        H: SystemHandler<C, M> + Send + 'static,
    {
        HandlerBuilder::new(self, addr.into(), id.into(), Box::new(handler))
    }

    /// Get the address of a previously added SystemHandler
    pub fn get_addr(&self, id: impl Into<String>) -> Option<Address> {
        self.inner.get(&id.into()).map(|data| data.addr.clone())
    }

    /// Specify an entry-point for the soon-to-be-built system
    pub fn set_entry<A: Into<Address>>(&mut self, addr: A) {
        self.entry = Some(addr.into());
    }

    /// Add a new handler to the builder, re-addressing the previous
    /// handler's "default" route to this one
    pub fn chain_default<A, H>(
        &mut self,
        addr: A,
        prev_id: impl Into<String>,
        id: impl Into<String>,
        handler: H,
    ) -> HandlerBuilder<'_, C, M>
    where
        A: Into<Address>,
        H: SystemHandler<C, M> + Send + 'static,
    {
        self.chain_for("default", addr, prev_id, id, handler)
    }

    /// Add a new handler to the builder, re-addressing the previous
    /// handler's named route to this one
    pub fn chain_for<A, H>(
        &mut self,
        rule: impl Into<String>,
        addr: A,
        prev_id: impl Into<String>,
        id: impl Into<String>,
        handler: H,
    ) -> HandlerBuilder<'_, C, M>
    where
        A: Into<Address>,
        H: SystemHandler<C, M> + Send + 'static,
    {
        let addr = addr.into();
        let prev_id = prev_id.into();
        if let Some(ref mut prev) = self.inner.get_mut(&prev_id) {
            prev.routes.insert(rule.into(), addr.clone());
        }
        self.add(addr, id, handler)
    }

    /// Re-address every system handler in order to clone it
    ///
    /// When initialising a worker system builder you setup handlers
    /// and routes between them.  However when cloning the worker
    /// system all addresses need to be re-addressed in order to allow
    /// a second instance of every address to exist on the same node.
    ///
    /// This problem may be solved by having scoped addresses in the
    /// future.
    ///
    /// This function assigns a new address to every handler and then
    /// changes routes that point to the old addresses.  Any route
    /// that is not in the internal set (i.e. the fin address routes)
    /// are replaced with the externally provided fin-address.
    pub fn readdress(&mut self, fin_addr: &Address) {
        // Generate a lookup-table first with new addresses to replace
        // the old addresses
        let addrs: BTreeMap<_, _> = self
            .inner
            .iter()
            .map(|(_, data)| (data.addr.clone(), Address::random(0)))
            .collect();

        // Small utility function that will either return the the new
        // address if it is within the system, or the new provided fin
        // address if it external (this only supports _one_ fin
        // endpoint)
        fn get_addr(
            addrs: &BTreeMap<Address, Address>,
            prev: &Address,
            fin_addr: &Address,
        ) -> Address {
            addrs.get(prev).unwrap_or(fin_addr).clone()
        }

        // Fix the entry point
        if let Some(entry) = core::mem::replace(&mut self.entry, None) {
            self.entry = addrs.get(&entry).cloned();
        }

        self.inner.iter_mut().for_each(|(_, data)| {
            data.addr = get_addr(&addrs, &data.addr, fin_addr);
            data.routes.iter_mut().for_each(|(_id, route_addr)| {
                *route_addr = get_addr(&addrs, route_addr, fin_addr);
            });
        });
    }

    /// Create a `WorkerSystem` and pre-initialise every SystemHandler
    pub async fn finalise(self, ctx: &mut C) -> Result<WorkerSystem<C, M>> {
        let mut system = WorkerSystem::default();
        if let Some(addr) = self.entry {
            system.set_entry(addr);
        }

        for (
            _identifier,
            HandlerData {
                addr,
                mut inner,
                mut routes,
            },
        ) in self.inner
        {
            inner.initialize(ctx, &mut routes).await?;
            system.attach_boxed(addr, inner);
        }

        Ok(system)
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
    addr: Option<Address>,
    id: Option<String>,
    inner: Option<Box<dyn SystemHandler<C, M> + Send + 'static>>,
    parent: &'paren mut SystemBuilder<C, M>,
}

impl<'paren, C, M> HandlerBuilder<'paren, C, M>
where
    C: Send + 'static,
    M: Message,
{
    fn new(
        parent: &'paren mut SystemBuilder<C, M>,
        addr: Address,
        id: String,
        inner: Box<dyn SystemHandler<C, M> + Send + 'static>,
    ) -> Self {
        Self {
            routes: Some(BTreeMap::new()),
            inner: Some(inner),
            id: Some(id),
            addr: Some(addr),
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
        let addr = core::mem::replace(&mut self.addr, None).unwrap();
        let id = core::mem::replace(&mut self.id, None).unwrap();

        assert_ne!(routes.len(), 0);
        self.parent.inner.insert(
            id,
            HandlerData {
                addr,
                inner,
                routes,
            },
        );
    }
}
