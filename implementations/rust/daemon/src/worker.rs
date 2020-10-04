use std::sync::mpsc::{self, Receiver, Sender};

use ockam_common::commands::ockam_commands::*;

pub struct Worker {
    router_tx: Sender<OckamCommand>,
    rx: Receiver<OckamCommand>,
}

impl Worker {
    fn new(router_tx: Sender<OckamCommand>, rx: Receiver<OckamCommand>) -> Self {}
}
