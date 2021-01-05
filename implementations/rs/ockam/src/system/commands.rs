#[allow(unused)]
//pub mod commands {
use crate::message::*;
use ockam_vault::Secret;
use std::sync::Arc;

#[derive(Debug)]
pub enum OckamCommand {
    Transport(TransportCommand),
    Router(RouterCommand),
    Channel(ChannelCommand),
    Worker(WorkerCommand),
}

// Transport commands - these can
// be sent to the transport_tx
#[derive(Debug)]
pub enum TransportCommand {
    Stop,
    SendMessage(Message),
}

// Router commands - these can be sent to the
// router_tx
#[derive(Debug)]
pub enum RouterCommand {
    Stop,
    Register(AddressType, std::sync::mpsc::Sender<OckamCommand>),
    SendMessage(Message),
    ReceiveMessage(Message),
}

// Channel commands - these can be sent to the
// channel_tx
#[derive(Debug)]
pub enum ChannelCommand {
    Initiate(Route, Address, Option<Arc<Box<dyn Secret>>>), /* route to destination, return
                                                             * local
                                                             * address */
    SendMessage(Message),
    ReceiveMessage(Message),
    Stop,
}

#[derive(Debug)]
pub enum WorkerCommand {
    Stop,
    Test,
    AddLine(String),
    SendPayload(String),
    ReceiveMessage(Message),
    SendMessage(Message),
}
//}
