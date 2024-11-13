#![allow(dead_code)]

use crate::Context;
use ockam_core::RelayMessage;

#[cfg(feature = "debugger")]
use ockam_core::{Address, Mailbox, Mailboxes};

#[cfg(feature = "debugger")]
use ockam_core::compat::{
    sync::{Arc, RwLock},
    vec::Vec,
};

#[cfg(feature = "debugger")]
use core::{
    mem::MaybeUninit,
    sync::atomic::{AtomicU32, Ordering},
};

#[cfg(feature = "debugger")]
#[derive(Default)]
struct Debugger {
    /// Map context inheritance from parent main `Mailbox` to child [`Mailboxes`]
    inherited_mb: Arc<RwLock<HashMap<Mailbox, Vec<Mailboxes>>>>,
    /// Map message destination to source
    incoming: Arc<RwLock<HashMap<Address, Vec<Address>>>>,
    /// Map message destination `Mailbox` to source [`Mailbox`]
    incoming_mb: Arc<RwLock<HashMap<Mailbox, Vec<Address>>>>,
    /// Map message source to destinations
    outgoing: Arc<RwLock<HashMap<Address, Vec<Address>>>>,
}

/// Return a mutable reference to the global debugger instance
/// TODO are there any better options for singletons yet that are also
/// no_std compatible?
#[cfg(feature = "debugger")]
#[allow(unsafe_code)]
fn instance() -> &'static Debugger {
    static mut INSTANCE: MaybeUninit<Debugger> = MaybeUninit::uninit();

    #[cfg(feature = "std")]
    {
        use std::sync::Once;
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let instance = Debugger::default();
            unsafe { INSTANCE.write(instance) };
        });
    }

    #[cfg(not(feature = "std"))]
    {
        use ockam_core::compat::sync::Mutex;
        static ONCE: Mutex<bool> = Mutex::new(true);
        if let Ok(mut once) = ONCE.lock() {
            if *once {
                let instance = Debugger::default();
                unsafe {
                    INSTANCE.write(instance);
                }
                *once = false;
            }
        } else {
            panic!("Failed to acquire initialization lock for debugger");
        }
    }

    unsafe { INSTANCE.assume_init_ref() }
}

/// Log incoming message traffic
///
/// This debug function builds a map of message traffic within a node.
///
/// Useful for:
///
/// 1. Figuring out the minimal set of access control rules for nodes
///    to communicate to each other.
/// 2. Understanding the ockam source code.
///
pub fn log_incoming_message(_receiving_ctx: &Context, _relay_msg: &RelayMessage) {
    #[cfg(feature = "debugger")]
    {
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        tracing::trace!(
            "log_incoming_message #{:03}: {} -> {} ({})",
            COUNTER.fetch_add(1, Ordering::Relaxed),
            _relay_msg.source(),              // sending address
            _relay_msg.destination(),         // receiving address
            _receiving_ctx.primary_address(), // actual receiving context address
        );

        match instance().incoming.write() {
            Ok(mut incoming) => {
                let source = _relay_msg.source().clone();
                let destination = _relay_msg.destination().clone();
                incoming
                    .entry(destination)
                    .or_insert_with(Vec::new)
                    .push(source);
            }
            Err(e) => {
                tracing::error!("debugger panicked: {}", e);
                panic!("log_incoming_message");
            }
        }

        match instance().incoming_mb.write() {
            Ok(mut incoming_mb) => {
                let source = _relay_msg.source().clone();
                let destination = _relay_msg.destination().clone();
                if let Some(destination_mb) = _receiving_ctx.mailboxes().find_mailbox(&destination)
                {
                    incoming_mb
                        .entry(destination_mb.clone())
                        .or_insert_with(Vec::new)
                        .push(source);
                }
            }
            Err(e) => {
                tracing::error!("debugger panicked: {}", e);
                panic!("log_incoming_message");
            }
        }
    }
}

/// Log outgoing message traffic
pub fn log_outgoing_message(_sending_ctx: &Context, _relay_msg: &RelayMessage) {
    #[cfg(feature = "debugger")]
    {
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        tracing::trace!(
            "log_outgoing_message #{:03}: {} ({}) -> {}",
            COUNTER.fetch_add(1, Ordering::Relaxed),
            _relay_msg.source(),            // sending address
            _sending_ctx.primary_address(), // actual sending context address
            _relay_msg.destination(),       // receiving address
        );

        match instance().outgoing.write() {
            Ok(mut outgoing) => {
                let source = _relay_msg.source().clone();
                let destination = _relay_msg.destination().clone();
                outgoing
                    .entry(source)
                    .or_insert_with(Vec::new)
                    .push(destination);
            }
            Err(e) => {
                tracing::error!("debugger panicked: {}", e);
                panic!("log_incoming_message");
            }
        }
    }
}

/// Log Context creation
///
/// This debug function builds an inheritance tree of the contexts
/// within a node.
///
/// Useful for:
///
/// 1. Figuring out the access control inheritance structure for a
///    node.
/// 2. Getting a rough idea of the "worker context" for a group of
///    contexts created by a top-level worker or processor interface
/// 3. Tracking down "orphan" contexts that could be vulnerable to
///    hostile messages
pub fn log_inherit_context(_tag: &str, _parent: &Context, _child: &Context) {
    #[cfg(feature = "debugger")]
    {
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        tracing::trace!(
            "log_inherit_context #{:03}\n{:?}\nBegat {}\n{:?}\n",
            COUNTER.fetch_add(1, Ordering::Relaxed),
            _parent.mailboxes(),
            _tag,
            _child.mailboxes(),
        );

        match instance().inherited_mb.write() {
            Ok(mut inherited_mb) => {
                let parent = _parent.mailboxes().primary_mailbox().clone();
                let children = _child.mailboxes().clone();
                inherited_mb
                    .entry(parent)
                    .or_insert_with(Vec::new)
                    .push(children);
            }
            Err(e) => {
                tracing::error!("debugger panicked: {}", e);
                panic!("log_incoming_message");
            }
        }
    }
}

/// TODO
pub fn _log_start_worker() {
    #[cfg(feature = "debugger")]
    {}
}

/// TODO
pub fn _log_start_processor() {
    #[cfg(feature = "debugger")]
    {}
}

// ----------------------------------------------------------------------------

#[cfg(all(feature = "debugger", feature = "std"))]
use ockam_core::compat::io::{self, BufWriter, Write};

/// Generate diagrams of the data logged by the Debugger
///
/// Diagram files can be rendered using graphviz, for example:
///
///    dot 07-inlet.dot -Tpdf -O
///    dot 07-inlet.dot -Tpdf -o 07-inlet.pdf
#[cfg(all(feature = "debugger", feature = "std"))]
pub fn generate_graphs<W: Write>(w: &mut BufWriter<W>) -> io::Result<()> {
    fn id(mailbox: &Mailbox) -> String {
        mailbox.address().address().replace('.', "_")
    }

    fn write_mailbox<W: Write>(
        w: &mut BufWriter<W>,
        mailbox: &Mailbox,
        tag: &str,
    ) -> io::Result<()> {
        write!(
            w,
            "    {}{} [label=\"{{ {} | in: {:?} | out: {:?}  }} \"]",
            tag,
            id(mailbox),
            mailbox.address(),
            mailbox.incoming_access_control(),
            mailbox.outgoing_access_control(),
        )?;
        writeln!(w)?;
        Ok(())
    }

    // generate mailboxes set
    use ockam_core::compat::collections::HashSet;
    let mut mailboxes = HashSet::new();
    if let Ok(inherited_mb) = instance().inherited_mb.read() {
        for (parent, children) in inherited_mb.iter() {
            for child in children.iter() {
                mailboxes.insert(parent.clone());
                mailboxes.insert(child.main_mailbox().clone());
                for mailbox in child.additional_mailboxes().iter() {
                    mailboxes.insert(mailbox.clone());
                }
            }
        }
    }

    writeln!(w, "digraph ockam_node {{")?;
    writeln!(w, "  fontname=Arial;")?;
    writeln!(w, "  rankdir=TB;")?;

    // - inheritance ----------------------------------------------------------
    writeln!(w, "  subgraph cluster_Inheritance {{")?;
    writeln!(w, "    label=\"Inheritance\";")?;
    writeln!(w, "    fontsize=24.0;")?;
    writeln!(w, "    labelloc=\"t\";")?;
    writeln!(w, "    rankdir=TB;")?;
    writeln!(w, "    edge [fillcolor=\"#a6cee3\"];")?;
    writeln!(w, "    edge [color=\"#1f78b4\"];")?;
    writeln!(w, "    node [shape=record];")?;
    writeln!(w, "    node [fontname=Arial];")?;
    writeln!(w, "    node [fontsize=12.0];")?;
    // metadata
    for mailbox in mailboxes.iter() {
        write_mailbox(w, mailbox, "")?;
    }
    // topology
    match instance().inherited_mb.read() {
        Ok(inherited_mb) => {
            for (parent, children) in inherited_mb.iter() {
                for child in children.iter() {
                    let mut child_ids = vec![id(child.main_mailbox())];
                    for mailbox in child.additional_mailboxes().iter() {
                        let child_id = id(mailbox);
                        child_ids.push(child_id);
                    }
                    for child_id in child_ids.iter() {
                        writeln!(w, "    {} -> {};", id(parent), child_id,)?;
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("debugger panicked: {}", e);
            panic!("display_log");
        }
    }
    writeln!(w, "  }}\n")?;

    // - message flow ---------------------------------------------------------
    writeln!(w, "  subgraph cluster_MessageFlow {{")?;
    writeln!(w, "    label=\"MessageFlow\";")?;
    writeln!(w, "    fontsize=24.0;")?;
    writeln!(w, "    fontname=Arial;")?;
    writeln!(w, "    labelloc=\"t\";")?;
    writeln!(w, "    rankdir=TB;")?;
    writeln!(w, "    edge [fillcolor=\"#a60000\"];")?;
    writeln!(w, "    edge [color=\"#1f0000\"];")?;
    writeln!(w, "    node [shape=Mrecord];")?;
    writeln!(w, "    node [fontname=Arial];")?;
    writeln!(w, "    node [fontsize=12.0];")?;
    // metadata
    for mailbox in mailboxes.iter() {
        write_mailbox(w, mailbox, "MF_")?;
    }
    match instance().incoming_mb.read() {
        Ok(incoming_mb) => {
            for (destination, sources) in incoming_mb.iter() {
                let mut sources = sources.clone();
                sources.sort();
                sources.dedup();
                for source in sources.iter() {
                    writeln!(
                        w,
                        "    MF_{} -> MF_{};",
                        //"    {} -> {};",
                        source.address().replace('.', "_"),
                        id(destination),
                    )?;
                }
            }
        }
        Err(e) => {
            tracing::error!("debugger panicked: {}", e);
            panic!("display_log");
        }
    }
    writeln!(w, "  }}")?;

    writeln!(w, "}}")?;
    w.flush()?;

    Ok(())
}

/// Displays a summary of the data logged by the Debugger
#[cfg(feature = "debugger")]
pub fn display_log() {
    tracing::info!("======================================================================");
    tracing::info!("  Contexts Inherited");
    tracing::info!("----------------------------------------------------------------------");
    match instance().inherited_mb.read() {
        Ok(inherited_mb) => {
            for (parent, children) in inherited_mb.iter() {
                tracing::info!("{:?}", parent);
                for child in children.iter() {
                    tracing::info!("    =>  {:?}", child);
                }
            }
        }
        Err(e) => {
            tracing::error!("debugger panicked: {}", e);
            panic!("display_log");
        }
    }

    tracing::info!("----------------------------------------------------------------------");
    tracing::info!("  Incoming Messages Received");
    tracing::info!("----------------------------------------------------------------------");
    /*match instance().incoming.read() {
        Ok(incoming) => {
            for (destination, sources) in incoming.iter() {
                let mut sources = sources.clone();
                sources.sort();
                sources.dedup();
                tracing::info!("{:40}  <=  {:?}", format!("{}", destination), sources);
            }
        }
        Err(e) => {
            tracing::error!("debugger panicked: {}", e);
            panic!("display_log");
        }
    }
    tracing::info!("----------------------------------------------------------------------");*/
    match instance().incoming_mb.read() {
        Ok(incoming_mb) => {
            for (destination, sources) in incoming_mb.iter() {
                tracing::info!("{:?}", destination);
                let mut sources = sources.clone();
                sources.sort();
                sources.dedup();
                for source in sources.iter() {
                    tracing::info!("    <=  {:?}", source);
                }
            }
        }
        Err(e) => {
            tracing::error!("debugger panicked: {}", e);
            panic!("display_log");
        }
    }

    /*tracing::info!("----------------------------------------------------------------------");
    tracing::info!("  Outgoing Messages Sent");
    tracing::info!("----------------------------------------------------------------------");
    match instance().outgoing.read() {
        Ok(outgoing) => {
            for (origin, destinations) in outgoing.iter() {
                let mut destinations = destinations.clone();
                destinations.sort();
                destinations.dedup();
                tracing::info!("{:40}  =>  {:?}", format!("{}", origin), destinations);
            }
        }
        Err(e) => {
            tracing::error!("debugger panicked: {}", e);
            panic!("display_log");
        }
    }*/
}
