use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::Address;

#[derive(Default)]
struct InternalRegistry {
    portal_workers: Vec<Address>,
    portal_receiver_processors: Vec<Address>,
    inlet_listener_processors: Vec<Address>,
    outlet_listener_workers: Vec<Address>,
    listener_processors: Vec<Address>,
    sender_workers: Vec<Address>,
    receiver_processors: Vec<Address>,
}

impl InternalRegistry {
    fn add_portal_worker(&mut self, addr: &Address) {
        self.portal_workers.push(addr.clone())
    }
    fn remove_portal_worker(&mut self, addr: &Address) {
        if let Some(index) = self.portal_workers.iter().position(|x| x == addr) {
            self.portal_workers.remove(index);
        }
    }
    fn add_portal_receiver_processor(&mut self, addr: &Address) {
        self.portal_receiver_processors.push(addr.clone())
    }
    fn remove_portal_receiver_processor(&mut self, addr: &Address) {
        if let Some(index) = self
            .portal_receiver_processors
            .iter()
            .position(|x| x == addr)
        {
            self.portal_receiver_processors.remove(index);
        }
    }
    fn add_inlet_listener_processor(&mut self, addr: &Address) {
        self.inlet_listener_processors.push(addr.clone())
    }
    fn remove_inlet_listener_processor(&mut self, addr: &Address) {
        if let Some(index) = self
            .inlet_listener_processors
            .iter()
            .position(|x| x == addr)
        {
            self.inlet_listener_processors.remove(index);
        }
    }
    fn add_outlet_listener_worker(&mut self, addr: &Address) {
        self.outlet_listener_workers.push(addr.clone())
    }
    fn remove_outlet_listener_worker(&mut self, addr: &Address) {
        if let Some(index) = self.outlet_listener_workers.iter().position(|x| x == addr) {
            self.outlet_listener_workers.remove(index);
        }
    }
    fn add_listener_processor(&mut self, addr: &Address) {
        self.listener_processors.push(addr.clone())
    }
    fn remove_listener_processor(&mut self, addr: &Address) {
        if let Some(index) = self.listener_processors.iter().position(|x| x == addr) {
            self.listener_processors.remove(index);
        }
    }
    fn add_sender_worker(&mut self, addr: &Address) {
        self.sender_workers.push(addr.clone())
    }
    fn remove_sender_worker(&mut self, addr: &Address) {
        if let Some(index) = self.sender_workers.iter().position(|x| x == addr) {
            self.sender_workers.remove(index);
        }
    }
    fn add_receiver_processor(&mut self, addr: &Address) {
        self.receiver_processors.push(addr.clone())
    }
    fn remove_receiver_processor(&mut self, addr: &Address) {
        if let Some(index) = self.receiver_processors.iter().position(|x| x == addr) {
            self.receiver_processors.remove(index);
        }
    }
}

/// Registry of all active workers and processors in TCP Transport
#[derive(Default, Clone)]
pub struct TcpRegistry {
    registry: Arc<RwLock<InternalRegistry>>,
}

impl TcpRegistry {
    pub(crate) fn add_portal_worker(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.add_portal_worker(addr);
        }
    }
    pub(crate) fn remove_portal_worker(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.remove_portal_worker(addr);
        }
    }
    pub(crate) fn add_portal_receiver_processor(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.add_portal_receiver_processor(addr);
        }
    }
    pub(crate) fn remove_portal_receiver_processor(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.remove_portal_receiver_processor(addr);
        }
    }
    pub(crate) fn add_inlet_listener_processor(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.add_inlet_listener_processor(addr);
        }
    }
    pub(crate) fn remove_inlet_listener_processor(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.remove_inlet_listener_processor(addr);
        }
    }
    pub(crate) fn add_outlet_listener_worker(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.add_outlet_listener_worker(addr);
        }
    }
    pub(crate) fn remove_outlet_listener_worker(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.remove_outlet_listener_worker(addr);
        }
    }
    pub(crate) fn add_listener_processor(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.add_listener_processor(addr);
        }
    }
    pub(crate) fn remove_listener_processor(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.remove_listener_processor(addr);
        }
    }
    pub(crate) fn add_sender_worker(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.add_sender_worker(addr);
        }
    }
    pub(crate) fn remove_sender_worker(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.remove_sender_worker(addr);
        }
    }
    pub(crate) fn add_receiver_processor(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.add_receiver_processor(addr);
        }
    }
    pub(crate) fn remove_receiver_processor(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.remove_receiver_processor(addr);
        }
    }
}
