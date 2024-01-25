//! Nodemanager API types

use minicbor::{Decode, Encode};

///////////////////-!  RESPONSE BODIES

/// Response body for a node status
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct NodeStatus {
    #[n(1)] pub node_name: String,
    #[n(2)] pub status: String,
    #[n(3)] pub workers: u32,
    #[n(4)] pub pid: i32,
}

impl NodeStatus {
    pub fn new(
        node_name: impl Into<String>,
        status: impl Into<String>,
        workers: u32,
        pid: i32,
    ) -> Self {
        Self {
            node_name: node_name.into(),
            status: status.into(),
            workers,
            pid,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::nodes::models::base::NodeStatus;
    use crate::schema::tests::validate_with_schema;
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};
    quickcheck! {
        fn project(n: NodeStatus) -> TestResult {
            validate_with_schema("node_status", n)
        }
    }

    impl Arbitrary for NodeStatus {
        fn arbitrary(g: &mut Gen) -> Self {
            NodeStatus {
                node_name: String::arbitrary(g),
                status: String::arbitrary(g),
                workers: u32::arbitrary(g),
                pid: i32::arbitrary(g),
            }
        }
    }
}
