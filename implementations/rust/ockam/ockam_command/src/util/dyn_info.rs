use colorful::Colorful;
use std::fmt::{self, Write as _};
#[derive(Debug, Clone)]
pub struct DynNodeInfo<'a> {
    name: &'a str,
    status: &'a str,
    services: Vec<NodeService<'a>>,
    secure_channel_addr_listener: &'a str,
}

#[derive(Debug, Clone)]
pub struct NodeService<'a> {
    service_type: ServiceType,
    address: &'a str,
    route: Option<&'a str>,
    identity: Option<&'a str>,
    auth_identity: Option<Vec<&'a str>>,
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

impl<'a> DynNodeInfo<'a> {
    /// Name of the Node
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            status: "",
            services: Vec::new(),
            secure_channel_addr_listener: "",
        }
    }
    /// Status can either be UP, DOWN, TODO
    pub fn status(mut self, status: &'a str) -> Self {
        self.status = status;
        self
    }

    /// Use NodeService::new()
    pub fn service(mut self, service: NodeService<'a>) -> Self {
        self.services.push(service);
        self
    }

    pub fn secure_channel_addr_listener(mut self, addr: &'a str) -> Self {
        self.secure_channel_addr_listener = addr;
        self
    }
}

impl<'a> NodeService<'a> {
    pub fn new(
        service_type: ServiceType,
        address: &'a str,
        route: Option<&'a str>,
        identity: Option<&'a str>,
        auth_identity: Option<Vec<&'a str>>,
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

impl<'a> fmt::Display for DynNodeInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut format_services = String::new();
        for service in &self.services {
            format_services = format_services + &format!("{}", service);
        }

        write!(
            f,
            "Node:\n\tName: {}\n\tStatus: {}\n\tServices:\n{}\tSecure Channel Listener Address:{}",
            self.name,
            match self.status {
                "UP" => self.status.light_green(),
                "DOWN" => self.status.light_red(),
                _ => self.status.white(),
            },
            format_services,
            self.secure_channel_addr_listener,
        )
    }
}

impl<'a> fmt::Display for NodeService<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = format!(
            "\t\tService:\n\t\t\tType: {}\n\t\t\tAddress: {}\n",
            self.service_type, self.address
        );
        if let (Some(route), Some(identity), Some(auth_identity)) =
            (self.route, self.identity, self.auth_identity.clone())
        {
            let mut format_auth_identity = String::new();
            for iden in auth_identity {
                format_auth_identity = format_auth_identity + &format!("\t\t\t\t- {}\n", iden);
            }

            let _ = write!(
                out,
                "\t\t\tRoute: {}\n\t\t\tIdentity: {}\n\t\t\tAuthorized Identities: \n{}",
                route, identity, format_auth_identity
            );
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
