#![allow(unsafe_code)]

use crate::privileged_portal::{
    Iface, InletRegistry, OutletRegistry, Port, Proto, RawSocketProcessor, TcpPacketWriter,
};
use aya::maps::{MapData, MapError};
use aya::programs::tc::SchedClassifierLink;
use aya::programs::{tc, Link, ProgramError, SchedClassifier, TcAttachType};
use aya::{Ebpf, EbpfError};
use aya_log::EbpfLogger;
use core::fmt::{Debug, Formatter};
use ockam_core::compat::collections::HashMap;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Address, Error, Result};
use ockam_node::compat::asynchronous::Mutex as AsyncMutex;
use ockam_node::Context;
use ockam_transport_core::TransportError;
use rand::random;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// eBPF support for [`TcpTransport`]
#[derive(Clone)]
pub struct TcpTransportEbpfSupport {
    pub(crate) ip_proto: u8,

    pub(crate) inlet_registry: InletRegistry,
    pub(crate) outlet_registry: OutletRegistry,

    links: Arc<Mutex<HashMap<Iface, IfaceLink>>>,

    tcp_packet_writer: Arc<AsyncMutex<Option<Box<dyn TcpPacketWriter>>>>,
    raw_socket_processor_address: Address,

    bpf: Arc<Mutex<Option<OckamBpf>>>,
}

struct IfaceLink {
    ingress: SchedClassifierLink,
    egress: SchedClassifierLink,
}

struct OckamBpf {
    ebpf: Ebpf,

    inlet_port_map: aya::maps::HashMap<MapData, Port, Proto>,
    outlet_port_map: aya::maps::HashMap<MapData, Port, Proto>,
}

impl Default for TcpTransportEbpfSupport {
    fn default() -> Self {
        let rnd: u16 = random();

        // Random in range [146, 252]
        let ip_proto = (146 + rnd % 107) as u8;

        Self {
            ip_proto,
            inlet_registry: Default::default(),
            outlet_registry: Default::default(),
            links: Default::default(),
            tcp_packet_writer: Default::default(),
            raw_socket_processor_address: Address::random_tagged("RawSocketProcessor"),
            bpf: Default::default(),
        }
    }
}

impl Debug for TcpTransportEbpfSupport {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "TcpTransportEbpfSupport")
    }
}

impl TcpTransportEbpfSupport {
    /// Start [`RawSocketProcessor`]. Should be done once.
    pub(crate) async fn start_raw_socket_processor_if_needed(
        &self,
        ctx: &Context,
    ) -> Result<Box<dyn TcpPacketWriter>> {
        debug!("Starting RawSocket");

        let mut tcp_packet_writer_lock = self.tcp_packet_writer.lock().await;
        if let Some(tcp_packet_writer_lock) = tcp_packet_writer_lock.as_ref() {
            return Ok(tcp_packet_writer_lock.create_new_box());
        }

        let (processor, tcp_packet_writer) = RawSocketProcessor::create(
            self.ip_proto,
            self.inlet_registry.clone(),
            self.outlet_registry.clone(),
        )
        .await?;

        *tcp_packet_writer_lock = Some(tcp_packet_writer.create_new_box());

        ctx.start_processor(self.raw_socket_processor_address.clone(), processor)
            .await?;

        info!("Started RawSocket for protocol: {}", self.ip_proto);

        Ok(tcp_packet_writer)
    }

    /// Start [`RawSocketProcessor`]. Should be done once.
    pub(crate) fn attach_ebpf_if_needed(&self, iface: Iface) -> Result<()> {
        self.init_ebpf()?;

        self.attach_ebpf(iface)?;

        Ok(())
    }

    /// Init eBPF system
    pub fn init_ebpf(&self) -> Result<()> {
        // FIXME: eBPF I doubt we can reuse that instance for different interfaces.
        let mut bpf_lock = self.bpf.lock().unwrap();
        if bpf_lock.is_some() {
            debug!("Skipping eBPF initialization");
            return Ok(());
        }

        debug!("Initializing eBPF");

        // Bump the memlock rlimit. This is needed for older kernels that don't use the
        // new memcg based accounting, see https://lwn.net/Articles/837122/
        let res = nix::sys::resource::setrlimit(
            nix::sys::resource::Resource::RLIMIT_MEMLOCK,
            nix::sys::resource::RLIM_INFINITY,
            nix::sys::resource::RLIM_INFINITY,
        );
        if let Some(err) = res.err() {
            warn!("remove limit on locked memory failed. Error: {}", err);
        }

        // This will include your eBPF object file as raw bytes at compile-time and load it at
        // runtime. This approach is recommended for most real-world use cases. If you would
        // like to specify the eBPF program at runtime rather than at compile-time, you can
        // reach for `Bpf::load_file` instead.

        let ebpf_binary = ockam_ebpf::EBPF_BINARY;
        let mut ebpf = Ebpf::load(ebpf_binary).map_err(map_ebpf_error)?;
        // eBPF can be read from the filesystem in the runtime for development purposes
        // let ebpf_binary = std::fs::read(PATH).unwrap();
        // let mut ebpf = Ebpf::load(&ebpf_binary).map_err(map_bpf_error)?;

        if let Err(e) = EbpfLogger::init(&mut ebpf) {
            // This can happen if you remove all log statements from your eBPF program.
            warn!("failed to initialize eBPF logger for ingress: {}", e);
        }

        let inlet_port_map = aya::maps::HashMap::<_, Port, Proto>::try_from(
            ebpf.take_map("INLET_PORT_MAP").unwrap(),
        )
        .map_err(map_map_error)?;
        let outlet_port_map = aya::maps::HashMap::<_, Port, Proto>::try_from(
            ebpf.take_map("OUTLET_PORT_MAP").unwrap(),
        )
        .map_err(map_map_error)?;

        let bpf = OckamBpf {
            ebpf,
            inlet_port_map,
            outlet_port_map,
        };

        *bpf_lock = Some(bpf);

        info!("Initialized eBPF");

        Ok(())
    }

    /// Attach eBPF to both ingress and egress of the given interface
    pub fn attach_ebpf(&self, iface: String) -> Result<()> {
        // error adding clsact to the interface if it is already added is harmless
        // the full cleanup can be done with 'sudo tc qdisc del dev eth0 clsact'.
        let _ = tc::qdisc_add_clsact(&iface);

        let mut links = self.links.lock().unwrap();

        if links.contains_key(&iface) {
            return Ok(());
        }
        let skip_load = !links.is_empty();

        let mut bpf_lock = self.bpf.lock().unwrap();
        let bpf = bpf_lock.as_mut().unwrap();

        // TODO: eBPF Avoid loading multiple times
        let ingress_link = self.attach_ebpf_ingress(iface.clone(), bpf, skip_load)?;
        let egress_link = self.attach_ebpf_egress(iface.clone(), bpf, skip_load)?;

        links.insert(
            iface.clone(),
            IfaceLink {
                ingress: ingress_link,
                egress: egress_link,
            },
        );

        Ok(())
    }

    fn attach_ebpf_ingress(
        &self,
        iface: String,
        bpf: &mut OckamBpf,
        skip_load: bool,
    ) -> Result<SchedClassifierLink> {
        debug!("Attaching eBPF ingress to {}", iface);

        let program_ingress: &mut SchedClassifier = bpf
            .ebpf
            .program_mut("ockam_ingress")
            .unwrap()
            .try_into()
            .map_err(map_program_error)?;
        if !skip_load {
            program_ingress.load().map_err(map_program_error)?;
        }
        let link_id = program_ingress
            .attach(&iface, TcAttachType::Ingress)
            .map_err(map_program_error)?;
        let link_id = program_ingress
            .take_link(link_id)
            .map_err(map_program_error)?;

        info!("eBPF ingress attached to {}", iface);

        Ok(link_id)
    }

    fn attach_ebpf_egress(
        &self,
        iface: String,
        bpf: &mut OckamBpf,
        skip_load: bool,
    ) -> Result<SchedClassifierLink> {
        debug!("Attaching eBPF egress to {}", iface);

        let program_egress: &mut SchedClassifier = bpf
            .ebpf
            .program_mut("ockam_egress")
            .unwrap()
            .try_into()
            .map_err(map_program_error)?;
        if !skip_load {
            program_egress.load().map_err(map_program_error)?;
        }
        let link_id = program_egress
            .attach(&iface, TcAttachType::Egress)
            .map_err(map_program_error)?;
        let link_id = program_egress
            .take_link(link_id)
            .map_err(map_program_error)?;

        info!("eBPF egress attached to {}", iface);

        Ok(link_id)
    }

    /// Detach the eBPF.
    pub fn detach_ebpfs(&self) {
        for (_iface, link) in self.links.lock().unwrap().drain() {
            _ = link.ingress.detach();
            _ = link.egress.detach();
        }
    }

    /// Add inlet port
    pub fn add_inlet_port(&self, port: Port) -> Result<()> {
        let mut bpf = self.bpf.lock().unwrap();

        bpf.as_mut()
            .unwrap()
            .inlet_port_map
            .insert(port, self.ip_proto, 0)
            .map_err(|e| TransportError::AddingInletPort(e.to_string()))?;

        Ok(())
    }

    /// Remove inlet port
    pub fn remove_inlet_port(&self, port: Port) -> Result<()> {
        let mut bpf = self.bpf.lock().unwrap();

        bpf.as_mut()
            .unwrap()
            .inlet_port_map
            .remove(&port)
            .map_err(|e| TransportError::RemovingInletPort(e.to_string()))?;

        Ok(())
    }

    /// Add outlet port
    pub fn add_outlet_port(&self, port: Port) -> Result<()> {
        let mut bpf = self.bpf.lock().unwrap();

        bpf.as_mut()
            .unwrap()
            .outlet_port_map
            .insert(port, self.ip_proto, 0)
            .map_err(|e| TransportError::AddingOutletPort(e.to_string()))?;

        Ok(())
    }

    /// Remove outlet port
    pub fn remove_outlet_port(&self, port: Port) -> Result<()> {
        let mut bpf = self.bpf.lock().unwrap();

        bpf.as_mut()
            .unwrap()
            .outlet_port_map
            .remove(&port)
            .map_err(|e| TransportError::RemovingOutletPort(e.to_string()))?;

        Ok(())
    }

    /// Return the address of this Processor
    pub fn raw_socket_processor_address(&self) -> &Address {
        &self.raw_socket_processor_address
    }
}

#[track_caller]
fn map_ebpf_error(ebpf_error: EbpfError) -> Error {
    Error::new(Origin::Core, Kind::Io, ebpf_error)
}

#[track_caller]
fn map_program_error(program_error: ProgramError) -> Error {
    Error::new(Origin::Core, Kind::Io, program_error)
}

#[track_caller]
fn map_map_error(map_error: MapError) -> Error {
    Error::new(Origin::Core, Kind::Io, map_error)
}

#[cfg(test)]
// requires root to run
mod tests {
    use crate::privileged_portal::TcpTransportEbpfSupport;
    use ockam_core::Result;
    use ockam_node::Context;

    #[ignore]
    #[ockam_macros::test]
    async fn test_init(_ctx: &mut Context) -> Result<()> {
        let ebpf_support = TcpTransportEbpfSupport::default();

        ebpf_support.init_ebpf()?;

        Ok(())
    }

    #[ignore]
    #[ockam_macros::test]
    async fn test_attach(_ctx: &mut Context) -> Result<()> {
        let ebpf_support = TcpTransportEbpfSupport::default();

        ebpf_support.init_ebpf()?;

        ebpf_support.attach_ebpf("lo".to_string())?;

        Ok(())
    }
}
