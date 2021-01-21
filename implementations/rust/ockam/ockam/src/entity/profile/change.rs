// Variants of changes allowed in a change event.
#[derive(Clone, Debug)]
pub enum ProfileChange {}

// Variants of proofs that are allowed on a change event.
#[derive(Clone, Debug)]
pub enum ProfileChangeProof {}

#[derive(Clone, Debug)]
pub struct Changes(Vec<ProfileChange>);

impl AsRef<[ProfileChange]> for Changes {
    fn as_ref(&self) -> &[ProfileChange] {
        &self.0
    }
}

impl Default for Changes {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl Changes {
    pub fn new(changes: Vec<ProfileChange>) -> Self {
        Self(changes)
    }
}

#[derive(Clone, Debug)]
pub struct ProfileChangeEvent {
    changes: Changes,
    proofs: Vec<ProfileChangeProof>,
}

impl ProfileChangeEvent {
    pub fn changes(&self) -> &[ProfileChange] {
        self.changes.as_ref()
    }
    pub fn proofs(&self) -> &[ProfileChangeProof] {
        &self.proofs
    }
}

impl ProfileChangeEvent {
    pub fn new(changes: Changes, proofs: Vec<ProfileChangeProof>) -> Self {
        ProfileChangeEvent { changes, proofs }
    }
}

#[derive(Clone, Debug)]
pub struct ProfileChangeHistory(Vec<ProfileChangeEvent>);

impl ProfileChangeHistory {
    pub fn new(change_events: Vec<ProfileChangeEvent>) -> Self {
        Self(change_events)
    }
}

impl AsRef<[ProfileChangeEvent]> for ProfileChangeHistory {
    fn as_ref(&self) -> &[ProfileChangeEvent] {
        &self.0
    }
}

impl Default for ProfileChangeHistory {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
