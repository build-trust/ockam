use ockam_core::{LocalInfoIdentifier, Result, Route, SecureChannelMetadata};
use ockam_node::Context;

/// State shared between `TcpInletListenProcessor` and `TcpInlet` to allow manipulating its state
/// from outside the worker: update the route to the outlet or pause it.
#[derive(Debug, Clone)]
pub struct InletSharedState {
    route: Route,
    // Identifier of the other side
    // The identifier is always the same for the same route as is obtained from the first local
    // secure channel on the route. However, we should recheck that identifier hasn't changed
    // when updating the route.
    their_identifier: Option<LocalInfoIdentifier>,
    is_paused: bool,
    // Starts with 0 and increments each time when inlet updates the route to the outlet
    // (e.g. when reconnecting), this will allow outlet to figure out what is the most recent
    // return_route even if messages arrive out-of-order
    route_index: u32,
}

impl InletSharedState {
    pub async fn create(ctx: &Context, route: Route, is_paused: bool) -> Result<Self> {
        let their_identifier =
            if let Some(terminal) = ctx.find_terminal_address(route.clone()).await? {
                SecureChannelMetadata::from_terminal_address(&terminal)
                    .map(|m| m.their_identifier())
                    .ok()
            } else {
                None
            };

        Ok(Self {
            route,
            their_identifier,
            is_paused,
            route_index: 0,
        })
    }

    pub fn route(&self) -> &Route {
        &self.route
    }

    pub fn their_identifier(&self) -> Option<LocalInfoIdentifier> {
        self.their_identifier.clone()
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn route_index(&self) -> u32 {
        self.route_index
    }

    pub async fn update_route(&mut self, ctx: &Context, new_route: Route) -> Result<()> {
        let their_identifier =
            if let Some(terminal) = ctx.find_terminal_address(new_route.clone()).await? {
                SecureChannelMetadata::from_terminal_address(&terminal)
                    .map(|m| m.their_identifier())
                    .ok()
            } else {
                None
            };

        self.their_identifier = their_identifier;

        self.route = new_route;
        // Overflow here is very unlikely...
        self.route_index += 1;

        Ok(())
    }

    pub fn set_is_paused(&mut self, is_paused: bool) {
        self.is_paused = is_paused;
    }
}
