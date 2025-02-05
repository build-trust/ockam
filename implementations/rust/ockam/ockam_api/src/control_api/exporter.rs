pub fn main() {
    let schema = ockam_api::control_api::generate_schema()
        .to_yaml()
        .expect("Failed to generate schema");
    println!("{}", schema);
}
