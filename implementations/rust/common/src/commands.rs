#[allow(unused)]
pub mod commands {
    use ockam_message::message::{AddressType, Message};
    use std::thread;

    // Transport commands
    pub enum TransportCommands {
        Stop,
        Add(String),
        Send(Message),
    }

    // Router commands
    pub enum RouterCommand {
        Stop,
        Register(AddressType, std::sync::mpsc::Sender<RouterCommand>),
        Route(Message),
    }

    // Channel commands
    pub enum ChannelCommand {
        Message(Message),
    }
}
