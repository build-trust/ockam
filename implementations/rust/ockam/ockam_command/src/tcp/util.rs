use crate::terminal::{OckamColor, Terminal, TerminalWriter};
use crate::Result;
use colorful::Colorful;
use miette::miette;
use ockam::Context;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;





use tokio::sync::Mutex;
use tokio::try_join;

pub fn alias_parser(arg: &str) -> Result<String> {
    if arg.contains(':') {
        Err(miette!("an alias must not contain ':' characters").into())
    } else {
        Ok(arg.to_string())
    }
}

pub async fn fetch_list<T: TerminalWriter, L: for<'b> minicbor::Decode<'b, ()>>(
    endpoint: &str,
    ctx: &Context,
    node: &BackgroundNode,
    terminal: &Terminal<T>,
) -> miette::Result<L> {
    let is_finished: Mutex<bool> = Mutex::new(false);

    let get_list = async {
        let items: L = node.ask(ctx, Request::get(endpoint)).await?;
        *is_finished.lock().await = true;
        Ok(items)
    };
    let output_messages = vec![format!(
        "Listing TCP Inlets on {}...\n",
        node.name().color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = terminal.progress_output(&output_messages, &is_finished);

    let (items, _) = try_join!(get_list, progress_output)?;
    Ok(items)
}
