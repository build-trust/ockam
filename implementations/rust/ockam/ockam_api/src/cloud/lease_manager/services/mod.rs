use minicbor::Decoder;
use ockam::Context;
use ockam_core::api::{Request, Response};
use ockam_core::{self, Result};

use crate::cloud::project;
use crate::nodes::NodeManagerWorker;

pub mod influxdb;

impl NodeManagerWorker {
    pub(crate) async fn handle_lease_request(
        &mut self,
        ctx: &mut Context,
        dec: &mut Decoder<'_>,
        req: &Request<'_>,
        project_id: &str,
        addon_id: &str,
    ) -> Result<Vec<u8>> {
        // TODO: Add on ids should not be magic strings
        match addon_id {
            "influxdb_token_lease_manager" => {
                self.handle_influxdb_lease_request(ctx, dec, req, project_id)
                    .await
            }
            _ => {
                let path = req.path();
                warn!(%path, "Called invalid endpoint");

                Ok(Response::bad_request(req.id())
                    .body(format!("Invalid endpoint: {}", path))
                    .to_vec()?)
            }
        }
    }
}
