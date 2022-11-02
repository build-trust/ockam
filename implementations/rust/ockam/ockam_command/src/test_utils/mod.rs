use assert_cmd::cargo::cargo_bin;
use once_cell::sync::OnceCell;

pub(crate) use cmd::{read_to_str, CmdBuilder};
pub(crate) use node::NodePool;

#[allow(unused)]
mod cmd;
mod node;

static OCKAM_BIN: OnceCell<String> = OnceCell::new();

fn ockam_bin() -> &'static str {
    OCKAM_BIN.get_or_init(|| format!("{}", cargo_bin("ockam").display()))
}
