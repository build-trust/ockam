use crate::Route;

use ockam_core::lib::Display;
use serde::{Deserialize, Serialize};
use std::fmt::{Formatter, Result};

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct RemoteEntity {
    pub route: Route,
}

impl Display for RemoteEntity {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.route.fmt(f)
    }
}

impl RemoteEntity {
    pub fn create<R: Into<Route>>(route: R) -> Self {
        RemoteEntity {
            route: route.into(),
        }
    }
}
