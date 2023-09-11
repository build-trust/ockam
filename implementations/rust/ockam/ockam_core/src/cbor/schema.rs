#[cfg(test)]
pub mod tests {
    use cddl_cat::validate_cbor_bytes;
    use minicbor::Encode;
    use quickcheck::TestResult;

    pub const SCHEMA: &str = core::include_str!("schema.cddl");

    pub fn validate_with_schema<T: Encode<()>>(rule_name: &str, t: T) -> TestResult {
        let cbor = minicbor::to_vec(t).unwrap();
        if let Err(e) = validate_cbor_bytes(rule_name, SCHEMA, &cbor) {
            return TestResult::error(e.to_string());
        }
        TestResult::passed()
    }
}
