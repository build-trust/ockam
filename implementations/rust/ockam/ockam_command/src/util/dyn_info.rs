pub struct DynInfo(String);

impl DynInfo {
    pub fn new_node() -> Self {
        DynInfo(String::from("Node:\n"))
    }

    pub fn name(mut self, name: &str) -> Self {
        let name = format!("  Name: {}\n", name);
        self.0.push_str(&name);
        self
    }

    pub fn status(mut self, status: &str) -> Self {
        let status = format!("  Status: {}\n", status);
        self.0.push_str(&status);
        self
    }

    pub fn services(mut self) -> Self {
        self.0.push_str("  Services:");
        self
    }

    pub fn service(mut self, r#type: &str, address: &str) -> Self {
        let service = format!(
            r#"
    Service: 
      Type: {}
      Address: {}"#,
            r#type, address
        );
        self.0.push_str(&service);
        self
    }

    pub fn service_detailed(
        mut self,
        r#type: &str,
        address: &str,
        route: &str,
        identity: &str,
        auth_identity: &str,
    ) -> Self {
        let service_detailed = format!(
            r#"
    Service:
      Type: {}
      Address: {}
      Route: {}
      Identity: {}
      Authorized Identities:
        - {}"#,
            r#type, address, route, identity, auth_identity
        );
        self.0.push_str(&service_detailed);
        self
    }

    pub fn scl(mut self) -> Self {
        self.0
            .push_str("\n  Secure Channel Listener Address: /service/api");
        self
    }
    pub fn build(self) -> String {
        self.0
    }
}
