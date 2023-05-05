use crate::{TcpListenerInfo, TcpReceiverInfo, TcpSenderInfo};
use ockam_core::Address;

#[derive(Default)]
pub(super) struct InternalRegistry {
    pub(super) portal_workers: Vec<Address>,
    pub(super) portal_receiver_processors: Vec<Address>,
    pub(super) inlet_listener_processors: Vec<Address>,
    pub(super) outlet_listener_workers: Vec<Address>,
    pub(super) listener_processors: Vec<TcpListenerInfo>,
    pub(super) sender_workers: Vec<TcpSenderInfo>,
    pub(super) receiver_processors: Vec<TcpReceiverInfo>,
}

impl InternalRegistry {
    pub(super) fn add_portal_worker(&mut self, addr: &Address) {
        self.portal_workers.push(addr.clone())
    }
    pub(super) fn remove_portal_worker(&mut self, addr: &Address) {
        self.portal_workers.retain(|x| x != addr);
    }
    pub(super) fn add_portal_receiver_processor(&mut self, addr: &Address) {
        self.portal_receiver_processors.push(addr.clone())
    }
    pub(super) fn remove_portal_receiver_processor(&mut self, addr: &Address) {
        self.portal_receiver_processors.retain(|x| x != addr);
    }
    pub(super) fn add_inlet_listener_processor(&mut self, addr: &Address) {
        self.inlet_listener_processors.push(addr.clone())
    }
    pub(super) fn remove_inlet_listener_processor(&mut self, addr: &Address) {
        self.inlet_listener_processors.retain(|x| x != addr);
    }
    pub(super) fn add_outlet_listener_worker(&mut self, addr: &Address) {
        self.outlet_listener_workers.push(addr.clone())
    }
    pub(super) fn remove_outlet_listener_worker(&mut self, addr: &Address) {
        self.outlet_listener_workers.retain(|x| x != addr);
    }
    pub(super) fn add_listener_processor(&mut self, info: TcpListenerInfo) {
        self.listener_processors.push(info)
    }
    pub(super) fn remove_listener_processor(&mut self, addr: &Address) {
        self.listener_processors.retain(|x| x.address() != addr);
    }
    pub(super) fn add_sender_worker(&mut self, info: TcpSenderInfo) {
        self.sender_workers.push(info)
    }
    pub(super) fn remove_sender_worker(&mut self, addr: &Address) {
        self.sender_workers.retain(|x| x.address() != addr);
    }
    pub(super) fn add_receiver_processor(&mut self, info: TcpReceiverInfo) {
        self.receiver_processors.push(info)
    }
    pub(super) fn remove_receiver_processor(&mut self, addr: &Address) {
        self.receiver_processors.retain(|x| x.address() != addr);
    }
}
