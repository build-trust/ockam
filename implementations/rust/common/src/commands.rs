#[allow(unused)]
pub mod ockam_commands {
    use ockam_message::{AddressType, Message, Route};
    use std::thread;

    pub enum OckamCommand {
        Transport(TransportCommand),
        Router(RouterCommand),
        Channel(ChannelCommand),
    }

    // Transport commands - these can
    // be sent to the transport_tx
    pub enum TransportCommand {
        Stop,
        Add(String, String),
        SendMessage(Message),
    }

    // Router commands - these can be sent to the
    // router_tx
    pub enum RouterCommand {
        Stop,
        Register(AddressType, std::sync::mpsc::Sender<OckamCommand>),
        SendMessage(Message),
        ReceiveMessage(Message),
    }

    // Channel commands - these can be sent to the
    // channel_tx
    pub enum ChannelCommand {
        Stop,
        InitializeRoute(Route),
        ReceiveMessage(Message),
        SendMessage(Message),
    }
}
