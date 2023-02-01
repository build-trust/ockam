use crate::access_control::IncomingAccessControl;
use crate::compat::{sync::Arc, vec::Vec};
use crate::{debugger, Address, DenyAll, OutgoingAccessControl, RelayMessage, Result};
use core::cmp::Ordering;
use core::fmt::{self, Debug};

/// A `Mailbox` controls the dispatch of incoming messages for a particular [`Address`]
/// Note that [`Worker`], [`Processor`] and [`Context`] may have multiple Mailboxes (with different
/// addresses), but they always have exactly one mpsc receiver (message queue)
#[derive(Clone)]
pub struct Mailbox {
    address: Address,
    incoming: Arc<dyn IncomingAccessControl>,
    outgoing: Arc<dyn OutgoingAccessControl>,
}

impl Debug for Mailbox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {{in:{:?} out:{:?}}}",
            self.address, self.incoming, self.outgoing
        )
    }
}

impl Ord for Mailbox {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.address.cmp(&rhs.address)
    }
}

impl PartialOrd for Mailbox {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

impl PartialEq for Mailbox {
    fn eq(&self, rhs: &Self) -> bool {
        self.address == rhs.address
    }
}

impl Eq for Mailbox {}

impl Mailbox {
    /// Create a new `Mailbox` with the given [`Address`], [`IncomingAccessControl`] and [`OutgoingAccessControl`]
    pub fn new(
        address: impl Into<Address>,
        incoming: Arc<dyn IncomingAccessControl>,
        outgoing: Arc<dyn OutgoingAccessControl>,
    ) -> Self {
        Self {
            address: address.into(),
            incoming,
            outgoing,
        }
    }

    /// Create a new `Mailbox` not allowed to send nor receive any messages
    pub fn deny_all(address: impl Into<Address>) -> Self {
        Self {
            address: address.into(),
            incoming: Arc::new(DenyAll),
            outgoing: Arc::new(DenyAll),
        }
    }

    /// Return a reference to the [`Address`] of this mailbox
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Return a reference to the [`IncomingAccessControl`] for this mailbox
    pub fn incoming_access_control(&self) -> &Arc<dyn IncomingAccessControl> {
        &self.incoming
    }

    /// Return a reference to the [`OutgoingAccessControl`] for this mailbox
    pub fn outgoing_access_control(&self) -> &Arc<dyn OutgoingAccessControl> {
        &self.outgoing
    }
}

/// A collection of [`Mailbox`]es for a specific [`Worker`], [`Processor`] or [`Context`]
#[derive(Clone)]
pub struct Mailboxes {
    main_mailbox: Mailbox,
    additional_mailboxes: Vec<Mailbox>,
}

impl Debug for Mailboxes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?} + {:?}",
            self.main_mailbox, self.additional_mailboxes
        )
    }
}

impl Mailboxes {
    /// Create [`Mailboxes`] given main [`Mailbox`] and collection of additional [`Mailbox`]es
    pub fn new(main_mailbox: Mailbox, additional_mailboxes: Vec<Mailbox>) -> Self {
        Self {
            main_mailbox,
            additional_mailboxes,
        }
    }

    /// Create [`Mailboxes`] with only main [`Mailbox`] for the given
    /// [`Address`] with [`IncomingAccessControl`] and [`OutgoingAccessControl`]
    pub fn main(
        address: impl Into<Address>,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Self {
        Self {
            main_mailbox: Mailbox::new(
                address.into(),
                incoming_access_control,
                outgoing_access_control,
            ),
            additional_mailboxes: vec![],
        }
    }

    /// Return all additional [`Address`]es represented by these [`Mailboxes`]
    pub fn additional_addresses(&self) -> Vec<Address> {
        self.additional_mailboxes
            .iter()
            .map(|x| x.address.clone())
            .collect()
    }

    /// Return the main [`Address`] of this [`Mailboxes`]
    pub fn main_address(&self) -> Address {
        self.main_mailbox.address.clone()
    }

    /// Return `true` if the given [`Address`] is included in this [`Mailboxes`]
    pub fn contains(&self, msg_addr: &Address) -> bool {
        if &self.main_mailbox.address == msg_addr {
            true
        } else {
            self.additional_mailboxes
                .iter()
                .any(|x| &x.address == msg_addr)
        }
    }

    /// Return a reference to the [`Mailbox`] with the given [`Address`]
    pub fn find_mailbox(&self, msg_addr: &Address) -> Option<&Mailbox> {
        if &self.main_mailbox.address == msg_addr {
            Some(&self.main_mailbox)
        } else {
            self.additional_mailboxes
                .iter()
                .find(|x| &x.address == msg_addr)
        }
    }

    /// Return `true` if the given [`RelayMessage`]
    /// is authorized to be received by this [`Mailboxes`]
    pub async fn is_incoming_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        if let Some(mailbox) = self.find_mailbox(relay_msg.destination()) {
            debugger::log_incoming_access_control(mailbox, relay_msg);

            mailbox.incoming.is_authorized(relay_msg).await
        } else {
            warn!(
                "Message from {} for {} does not match any addresses for this destination",
                relay_msg.source(),
                relay_msg.destination()
            );
            crate::deny()
        }
    }

    /// Return `true` if the given [`RelayMessage`]
    /// is authorized to be sent by this [`Mailboxes`]
    pub async fn is_outgoing_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        if let Some(mailbox) = self.find_mailbox(relay_msg.source()) {
            debugger::log_outgoing_access_control(mailbox, relay_msg);

            mailbox.outgoing.is_authorized(relay_msg).await
        } else {
            warn!(
                "Message from {} for {} does not match any addresses for this origin",
                relay_msg.source(),
                relay_msg.destination()
            );
            crate::deny()
        }
    }

    /// Return all (mail + additional) [`Address`]es represented by this [`Mailboxes`]
    pub fn addresses(&self) -> Vec<Address> {
        let mut addresses = vec![self.main_mailbox.address.clone()];
        addresses.append(&mut self.additional_addresses());
        addresses
    }

    /// Return a reference to the main [`Mailbox`] for this [`Mailboxes`]
    pub fn main_mailbox(&self) -> &Mailbox {
        &self.main_mailbox
    }

    /// Return a reference to the additional [`Mailbox`]es for this [`Mailboxes`]
    pub fn additional_mailboxes(&self) -> &Vec<Mailbox> {
        &self.additional_mailboxes
    }
}
