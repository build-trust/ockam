#[allow(unused)]
pub mod ockam_commands {
    use ockam_message::message::*;
    use std::thread;

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
        Stop,
        ReceiveMessage(Message),
        SendMessage(Message),
    }

    #[derive(Debug)]
    pub enum WorkerCommand {
        Stop,
        Test,
        ReceiveMessage(Message),
        SendMessage(Message),
    }
}
