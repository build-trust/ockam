use crate::{TcpListenerInfo, TcpReceiverInfo, TcpRegistry, TcpSenderInfo};
use ockam_core::Address;

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
    pub(crate) fn add_listener_processor(&self, info: TcpListenerInfo) {
        if let Ok(mut lock) = self.registry.write() {
            lock.add_listener_processor(info);
        }
    }
    pub(crate) fn remove_listener_processor(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.remove_listener_processor(addr);
        }
    }
    pub(crate) fn add_sender_worker(&self, info: TcpSenderInfo) {
        if let Ok(mut lock) = self.registry.write() {
            lock.add_sender_worker(info);
        }
    }
    pub(crate) fn remove_sender_worker(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.remove_sender_worker(addr);
        }
    }
    pub(crate) fn add_receiver_processor(&self, info: TcpReceiverInfo) {
        if let Ok(mut lock) = self.registry.write() {
            lock.add_receiver_processor(info);
        }
    }
    pub(crate) fn remove_receiver_processor(&self, addr: &Address) {
        if let Ok(mut lock) = self.registry.write() {
            lock.remove_receiver_processor(addr);
        }
    }
}
