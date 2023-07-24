use ockam_api::nodes::models::portal::OutletStatus;

#[derive(Default)]
pub(super) struct ModelState {
    pub(super) is_enrolled: bool,
    pub(super) outlets: Vec<OutletStatus>,
}

impl ModelState {
    pub(super) fn new(is_enrolled: bool, outlets: Vec<OutletStatus>) -> Self {
        Self {
            is_enrolled,
            outlets,
        }
    }

    pub(super) fn set_enrolled(&mut self) {
        self.is_enrolled = true
    }

    pub(super) fn add_outlet(&mut self, outlet: OutletStatus) {
        self.outlets.push(outlet)
    }
}
