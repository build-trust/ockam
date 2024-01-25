use minicbor::{Decode, Encode};

use ockam_core::flow_control::FlowControlId;
use ockam_multiaddr::MultiAddr;

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddConsumer {
    #[n(1)] flow_control_id: FlowControlId,
    #[n(2)] address: MultiAddr,
}

impl AddConsumer {
    pub fn new(flow_control_id: FlowControlId, address: MultiAddr) -> Self {
        Self {
            flow_control_id,
            address,
        }
    }
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
    pub fn address(&self) -> &MultiAddr {
        &self.address
    }
}

#[cfg(test)]
mod tests {
    use ockam_core::flow_control::FlowControlId;
    use ockam_multiaddr::MultiAddr;
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};

    use crate::nodes::models::flow_controls::AddConsumer;
    use crate::schema::tests::validate_with_schema;

    quickcheck! {
        fn add_consumer(g: AddConsumer) -> TestResult {
            validate_with_schema("add_consumer", g)
        }
    }

    impl Arbitrary for AddConsumer {
        fn arbitrary(g: &mut Gen) -> Self {
            AddConsumer {
                flow_control_id: FlowControlId::from(String::arbitrary(g)),
                address: MultiAddr::default(),
            }
        }
    }
}
