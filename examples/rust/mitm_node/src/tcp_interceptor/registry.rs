use crate::tcp_interceptor::Role;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::Address;
use ockam_node::compat::asynchronous::Mutex;
use tokio::net::tcp::OwnedWriteHalf;

#[derive(Clone)]
pub struct ProcessorInfo {
    pub address: Address,
    pub role: Role,
    pub write_half: Arc<Mutex<OwnedWriteHalf>>,
}

#[derive(Default, Clone)]
pub struct TcpMitmRegistry {
    registry: Arc<RwLock<InternalRegistry>>,
}

impl TcpMitmRegistry {
    pub(crate) fn add_processor(&self, addr: &Address, role: Role, write_half: Arc<Mutex<OwnedWriteHalf>>) {
        if let Ok(mut lock) = self.registry.write() {
            lock.add_processor(addr, role, write_half);
        }
    }
    pub(crate) fn remove_processor(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.remove_processor(addr);
        }
    }
    pub(crate) fn add_listener(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.add_listener(addr);
        }
    }
    pub(crate) fn remove_listener(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.remove_listener(addr);
        }
    }
}

impl TcpMitmRegistry {
    pub fn get_all_processors(&self) -> Vec<ProcessorInfo> {
        self.registry.read().unwrap().processors.clone()
    }
    pub fn get_all_listeners(&self) -> Vec<Address> {
        self.registry.read().unwrap().listeners.clone()
    }
}

#[derive(Default)]
struct InternalRegistry {
    processors: Vec<ProcessorInfo>,
    listeners: Vec<Address>,
}

impl InternalRegistry {
    fn add_processor(&mut self, addr: &Address, role: Role, write_half: Arc<Mutex<OwnedWriteHalf>>) {
        self.processors.push(ProcessorInfo {
            address: addr.clone(),
            role,
            write_half,
        })
    }
    fn remove_processor(&mut self, addr: &Address) {
        self.processors.retain(|x| &x.address != addr);
    }
    fn add_listener(&mut self, addr: &Address) {
        self.listeners.push(addr.clone())
    }
    fn remove_listener(&mut self, addr: &Address) {
        self.listeners.retain(|x| x != addr);
    }
}
