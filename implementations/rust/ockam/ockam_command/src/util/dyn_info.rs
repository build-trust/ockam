use colorful::Colorful;
use std::fmt;
#[derive(Debug, Clone)]
pub struct DynNodeInfo {
    name: String,
    status: String,
    services: Vec<NodeService>,
    secure_channel_addr_listener: String,
}

#[derive(Debug, Clone)]
pub struct NodeService {
    service_type: ServiceType,
    address: String,
    route: Option<String>,
    identity: Option<String>,
    auth_identity: Option<Vec<String>>,
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ServiceType {
    TCPConnection,
    TCPListener,
    SecureChannelConnection,
    SecureChannelListener,
    Uppercase,
    Echo,
}

// Can't accept arbitrary status
// #[derive(Debug)]
// pub enum Status {
//     UP,
//     DOWN,
//     NONE,
// }

impl DynNodeInfo {
    /// Name of the Node
    pub fn new(name: String) -> Self {
        Self {
            name,
            status: String::new(),
            services: Vec::new(),
            secure_channel_addr_listener: String::new(),
        }
    }
    /// Status can either be UP, DOWN, TODO
    pub fn status(mut self, status: String) -> Self {
        self.status = status;
        self
    }

    /// Use NodeService::new()
    pub fn service(mut self, service: NodeService) -> Self {
        self.services.push(service);
        self
    }

    pub fn secure_channel_addr_listener(mut self, addr: String) -> Self {
        self.secure_channel_addr_listener = addr;
        self
    }
}

impl NodeService {
    pub fn new(
        service_type: ServiceType,
        address: String,
        route: Option<String>,
        identity: Option<String>,
        auth_identity: Option<Vec<String>>,
    ) -> Self {
        Self {
            service_type,
            address,
            route,
            identity,
            auth_identity,
        }
    }
}

impl fmt::Display for DynNodeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut format_services = String::new();
        for service in &self.services {
            format_services = format_services + &format!("{}", service);
        }

        write!(
            f,
            "{}",
            format!(
                "Node:\n\tName: {}\n\tStatus: {}\n\tServices:\n{}\tSecure Channel Listener Address:{}",
                self.name,
                match self.status.as_str() {
                    "UP" => self.status.clone().light_green(),
                    "DOWN" => self.status.clone().light_red(),
                    _ => self.status.clone().white(),
                },
                format_services,
                self.secure_channel_addr_listener,
            )
        )
    }
}

impl fmt::Display for NodeService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = format!(
            "\t\tService:\n\t\t\tType: {}\n\t\t\tAddress: {}\n",
            self.service_type, self.address
        );
        match (
            self.route.clone(),
            self.identity.clone(),
            self.auth_identity.clone(),
        ) {
            (Some(route), Some(identity), Some(auth_identity)) => {
                let mut format_auth_identity = String::new();
                for iden in auth_identity {
                    format_auth_identity = format_auth_identity + &format!("\t\t\t\t- {}\n", iden);
                }
                out.push_str(&format!(
                    "\t\t\tRoute: {}\n\t\t\tIdentity: {}\n\t\t\tAuthorized Identities: \n{}",
                    route, identity, format_auth_identity
                ));
            }
            (_, _, _) => (),
        }
        write!(f, "{}", out)
    }
}

impl fmt::Display for ServiceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceType::TCPConnection => write!(f, "TCP Connection"),
            ServiceType::TCPListener => write!(f, "TCP Listener"),
            ServiceType::SecureChannelConnection => write!(f, "Secure Channel Connection"),
            ServiceType::SecureChannelListener => write!(f, "Secure Channel Listener"),
            ServiceType::Uppercase => write!(f, "Uppercase"),
            ServiceType::Echo => write!(f, "Echo"),
        }
    }
}
