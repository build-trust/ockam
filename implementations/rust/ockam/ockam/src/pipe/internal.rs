//! Internal messaging structures

use ockam_core::Route;
use serde::{Deserialize, Serialize};

/// Internal command issued to
#[derive(Serialize, Deserialize)]
pub struct CreatePipe {
    route_to_sender: Route,
}
