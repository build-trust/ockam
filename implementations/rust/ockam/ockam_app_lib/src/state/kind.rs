use crate::state::AppState;

/// Represents the kind of state that can be loaded
#[repr(u8)]
pub enum StateKind {
    Projects = 1 << 0,
    Invitations = 1 << 1,
}

impl StateKind {
    /// Returns a full bitmask of every state, useful to check if all the state has been loaded
    pub fn full_bitmask() -> u8 {
        Self::Projects as u8 | Self::Invitations as u8
    }
}

impl AppState {
    /// Mark a specific kind of state as loaded
    pub fn mark_as_loaded(&self, state_loaded: StateKind) {
        let mut guard = self.state_loaded.lock().unwrap();
        *guard |= state_loaded as u8;
    }

    /// Returns true when all the states has been loaded
    pub fn is_state_loaded(&self) -> bool {
        let guard = self.state_loaded.lock().unwrap();
        *guard == StateKind::full_bitmask()
    }
}
