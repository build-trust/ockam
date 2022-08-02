//! An automatic setup mechanism for composition snippets

#![allow(unused)]

use crate::util::{ComposableSnippet, Operation, RemoteMode};
use std::collections::VecDeque;
use std::{env::current_exe, path::PathBuf, process::Command};

pub fn run(node: &str, vec: &VecDeque<ComposableSnippet>) {
    let ockam = current_exe().unwrap_or_else(|_| "ockam".into());

    for snippet in vec {
        run_snippet(&ockam, node, snippet);
    }
}

pub fn run_foreground(node: &str, vec: &VecDeque<ComposableSnippet>) {
    let ockam = current_exe().unwrap_or_else(|_| "ockam".into());

    for snippet in vec {
        // When we run in foreground mode we skip the Node creation
        // step because it already exists and is waiting for us
        // (hopefully).
        if let Operation::Node { .. } = snippet.op {
            continue;
        }

        run_snippet(&ockam, node, snippet);
    }
}

fn run_snippet(
    ockam: &PathBuf,
    node_name: &str,
    snippet @ ComposableSnippet { id, op, params }: &ComposableSnippet,
) {
    let args = match op {
        Operation::Node {
            api_addr,
            node_name: _,
        } => vec!["node", "create", node_name, "--api-address", api_addr],
        Operation::Transport {
            mode,
            address,
            protocol: _,
        } => vec![
            "transport",
            "create",
            "--node",
            node_name,
            match mode {
                RemoteMode::Connector => "tcp-connector",
                RemoteMode::Listener => "tcp-listener",
                RemoteMode::Receiver => unimplemented!(),
            },
            address,
        ],
        Operation::Portal {
            mode,
            protocol,
            bind,
            peer,
        } => {
            todo!()
        }
        Operation::SecureChannel => {
            todo!()
        }

        Operation::Forwarder => {
            todo!()
        }
    };

    if let Err(e) = Command::new(ockam).args(args).output() {
        eprintln!("failed to execute snippet '{:?}': {}", snippet, e);
    }

    println!("ok");
}
