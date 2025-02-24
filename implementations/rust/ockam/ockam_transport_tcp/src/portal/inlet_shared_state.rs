use crate::TcpInletOptions;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{LocalInfoIdentifier, Result, Route, SecureChannelMetadata};
use ockam_node::Context;
use rand::seq::SliceRandom;
use std::sync::{Arc, Mutex as SyncMutex, RwLock as SyncRwLock};
use tokio::sync::Notify;
use tracing::debug;

/// State shared between `TcpInletListenProcessor` and `TcpInlet` to allow manipulating its state
/// from outside the worker: update the route to the outlet or pause it.
#[derive(Debug, Default, Clone)]
pub struct InletSharedState {
    routes: Arc<SyncRwLock<Vec<(String, InletRouteMutableState)>>>,
    // To notify the listener that a new route is available
    notify: Arc<Notify>,
}

impl InletSharedState {
    /// Add a new route to the shared state and return true if the route was added successfully.
    /// If the route already exists, the function will return false.
    pub fn try_add_route(&self, key: String, route_state: InletRouteMutableState) -> bool {
        let mut guard = self.routes.write().unwrap();
        if guard.iter().any(|(k, _)| k == &key) {
            false
        } else {
            guard.push((key, route_state));
            self.notify.notify_waiters();
            true
        }
    }

    /// Adds a new route to the shared state and return an error if the route already exists.
    pub fn add_route(&self, key: String, route_state: InletRouteMutableState) -> Result<()> {
        if !self.try_add_route(key, route_state) {
            Err(ockam_core::Error::new(
                Origin::Channel,
                Kind::AlreadyExists,
                "Route already exists",
            ))
        } else {
            Ok(())
        }
    }

    /// List all route keys
    pub fn list_all_route_keys(&self) -> Vec<String> {
        let guard = self.routes.read().unwrap();
        guard.iter().map(|(k, _)| k.clone()).collect()
    }

    /// Removes a route given its key
    pub fn remove_route(&self, route_key: &str) {
        let mut guard = self.routes.write().unwrap();
        guard.retain(|(k, _)| k != route_key);
    }

    /// Choose one random active route from the shared state, if no active routes are found, wait
    /// until a new route is added or an existing route is unpaused.
    pub async fn choose_active_route(&self) -> InletRouteMutableState {
        loop {
            {
                let guard = self.routes.read().unwrap();
                let active_routes: Vec<&InletRouteMutableState> = guard
                    .iter()
                    .filter(|(_, route)| !route.is_paused())
                    .map(|(_, route)| route)
                    .collect();

                if !active_routes.is_empty() {
                    break active_routes
                        .choose(&mut rand::thread_rng())
                        .copied()
                        .cloned()
                        .unwrap();
                }
            }
            // let's wait until something changes and try again
            debug!("No active route found for the inlet, waiting for a connection");
            self.notify.notified().await;
        }
    }

    /// Update the route and set pause to false
    pub fn update_route_and_unpause(
        &self,
        ctx: &Context,
        route_key: &str,
        new_route: Route,
        options: TcpInletOptions,
    ) -> Result<()> {
        debug!("Updating route {route_key} with {new_route}");

        let guard = self.routes.read().unwrap();
        if let Some(route) = guard.iter().find(|(k, _)| k == route_key) {
            route.1.update_route(ctx, new_route, options)?;
            self.notify.notify_waiters();
            Ok(())
        } else {
            Err(ockam_core::Error::new(
                Origin::Channel,
                Kind::NotFound,
                "Route Key not found",
            ))
        }
    }

    /// Pause the route
    pub fn pause(&self, route_key: &str) {
        let guard = self.routes.read().unwrap();
        if let Some(route) = guard.iter().find(|(k, _)| k == route_key) {
            route.1.pause();
        }
    }
}

#[derive(Clone, Debug)]
pub struct InletRouteMutableState {
    inner: Arc<SyncMutex<InletRouteState>>,
}

impl InletRouteMutableState {
    pub fn create(
        ctx: &Context,
        route: Route,
        is_paused: bool,
        options: TcpInletOptions,
    ) -> Result<Self> {
        let their_identifier =
            if let Some((_address, metadata)) = ctx.find_terminal_address(route.iter())? {
                SecureChannelMetadata::from_terminal_address_metadata(&metadata)
                    .map(|m| m.their_identifier())
                    .ok()
            } else {
                None
            };

        Ok(Self {
            inner: Arc::new(SyncMutex::new(InletRouteState {
                route,
                their_identifier,
                is_paused,
                route_index: 0,
                options,
            })),
        })
    }

    /// Create a consistent snapshot of the current route state
    pub fn snapshot(&self) -> InletRouteState {
        self.inner.lock().unwrap().clone()
    }

    /// Create a consistent snapshot of the current route index and route
    pub fn snapshot_route(&self) -> (u32, Route) {
        let guard = self.inner.lock().unwrap();
        (guard.route_index, guard.route.clone())
    }

    /// Returns the identifier of the other side of the secure channel.
    /// The identifier will remain the same regardless of the route changes
    pub fn their_identifier(&self) -> Option<LocalInfoIdentifier> {
        self.inner.lock().unwrap().their_identifier()
    }

    pub fn is_paused(&self) -> bool {
        self.inner.lock().unwrap().is_paused()
    }

    pub fn update_route(
        &self,
        ctx: &Context,
        new_route: Route,
        options: TcpInletOptions,
    ) -> Result<()> {
        self.inner
            .lock()
            .unwrap()
            .update_route(ctx, new_route, options)
    }

    pub fn pause(&self) {
        self.inner.lock().unwrap().is_paused = true;
    }
}

/// State of a single inlet route.
#[derive(Debug, Clone)]
pub struct InletRouteState {
    /// Route to the outlet
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

    // TODO: should options be dependent on the MultiAddr?
    options: TcpInletOptions,
}

impl InletRouteState {
    pub fn route(&self) -> &Route {
        &self.route
    }

    pub fn options(&self) -> &TcpInletOptions {
        &self.options
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

    fn update_route(
        &mut self,
        ctx: &Context,
        new_route: Route,
        options: TcpInletOptions,
    ) -> Result<()> {
        let their_identifier =
            if let Some((_address, metadata)) = ctx.find_terminal_address(new_route.iter())? {
                SecureChannelMetadata::from_terminal_address_metadata(&metadata)
                    .map(|m| m.their_identifier())
                    .ok()
            } else {
                None
            };

        if let Some(current_identifier) = &self.their_identifier {
            if let Some(new_identifier) = &their_identifier {
                if current_identifier != new_identifier {
                    return Err(ockam_core::Error::new(
                        Origin::Channel,
                        Kind::Conflict,
                        "Route identifier has changed",
                    ));
                }
            } else {
                return Err(ockam_core::Error::new(
                    Origin::Channel,
                    Kind::Conflict,
                    "Route identifier not found",
                ));
            }
        }

        self.their_identifier = their_identifier;
        self.options = options;

        self.route = new_route;
        // Overflow here is very unlikely...
        self.route_index += 1;
        self.is_paused = false;

        Ok(())
    }
}
