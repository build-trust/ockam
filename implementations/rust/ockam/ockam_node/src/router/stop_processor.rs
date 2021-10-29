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
    main_addr: &Address,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    trace!("Stopping processor '{}'", main_addr);

    let aux_addr = main_addr.suffix(PROC_ADDR_SUFFIX);

    // First check if a processor of this address exists and
    // remove both address records.  We can drop both records here
    // too.  For the main address this means that no more messages
    // can be sent to this processor, and for the aux address this
    // initiates processor shutdown.
    match (
        router.map.internal.remove(&aux_addr),
        router.map.internal.remove(main_addr),
    ) {
        (Some(_), Some(_)) => {}
        // If by any chance only one of the records existed we are
        // in an undefined router state and will panic (for now)
        (Some(_), None) | (None, Some(_)) => {
            panic!("Invalid router state: mismatching processor address records!")
        }
        _ => {
            reply
                .send(NodeReply::no_such_processor(main_addr.clone()))
                .await
                .map_err(|_| Error::InternalIOFailure)?;

            return Ok(());
        }
    };

    // Remove  main address from addr_map too
    router.map.addr_map.remove(main_addr);

    // Signal back that everything went OK
    reply
        .send(NodeReply::ok())
        .await
        .map_err(|_| Error::InternalIOFailure)?;

    Ok(())
}
