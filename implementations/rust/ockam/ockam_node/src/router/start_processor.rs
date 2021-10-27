use super::{AddressRecord, NodeState, Router};
use crate::tokio::sync::mpsc::Sender;
use crate::{
    error::Error,
    relay::{RelayMessage, PROC_ADDR_SUFFIX},
    NodeReply, NodeReplyResult,
};

use ockam_core::{Address, Result};

pub(super) async fn exec(
    router: &mut Router,
    addr: Address,
    main_sender: Sender<RelayMessage>,
    aux_sender: Sender<RelayMessage>,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    trace!("Starting new processor '{}'", &addr);

    let aux_addr = addr.suffix(PROC_ADDR_SUFFIX);

    let main_record = AddressRecord::new(addr.clone().into(), main_sender);
    let aux_record = AddressRecord::new(aux_addr.clone().into(), aux_sender);

    // We insert both records without a reference to each other
    // because when we stop the processor we can easily derive the
    // aux address via the well-known suffix.  If this is at some
    // point no longer sufficient, we should start adding the
    // addresses as full aliases via address-record or addr_map
    router.internal.insert(addr.clone(), main_record);
    router.internal.insert(aux_addr.clone(), aux_record);

    #[cfg(feature = "std")]
    if std::env::var("OCKAM_DUMP_INTERNALS").is_ok() {
        trace!("{:#?}", router.internal);
    }
    #[cfg(all(not(feature = "std"), feature = "dump_internals"))]
    trace!("{:#?}", router.internal);

    router.addr_map.insert(addr.clone(), addr.clone());

    // For now we just send an OK back -- in the future we need to
    // communicate the current executor state
    reply
        .send(NodeReply::ok())
        .await
        .map_err(|_| Error::InternalIOFailure)?;
    Ok(())
}
