use crate::contact::contact_event::ContactEvent;
use crate::contact::ContactVault;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type ContactTags = HashMap<String, String>;

#[derive(Clone)]
pub struct Contact {
    identifier: String,
    events: Vec<ContactEvent>,
    vault: Arc<Mutex<dyn ContactVault>>,
}

impl Contact {
    pub(crate) fn new(
        identifier: String,
        events: Vec<ContactEvent>,
        vault: Arc<Mutex<dyn ContactVault>>,
    ) -> Self {
        Contact {
            identifier,
            events,
            vault,
        }
    }
}

impl Contact {
    pub fn identifier(&self) -> &str {
        &self.identifier
    }
    pub fn events(&self) -> &Vec<ContactEvent> {
        &self.events
    }
    pub fn vault(&self) -> &Arc<Mutex<dyn ContactVault>> {
        &self.vault
    }
}
