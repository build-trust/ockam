pub(crate) mod connection;
pub(crate) mod inlet;
pub(crate) mod listener;
pub(crate) mod outlet;

#[cfg(test)]
mod tests {
    use crate::test_utils::{CmdBuilder, NodePool};
    use crate::util::find_available_port;
    use anyhow::Result;
    use assert_cmd::prelude::*;

    #[test]
    #[ignore]
    fn inlet_outlet_pair() -> Result<()> {
        let node_1 = NodePool::pull();
        let outlet_port = find_available_port()?;
        let cmd = CmdBuilder::ockam(&format!(
            "tcp-outlet create --at /node/{} --from /service/outlet --to 127.0.0.1:{outlet_port}",
            node_1.name()
        ))?;
        cmd.run()?.assert().success();

        let node_2 = NodePool::pull();
        let inlet_port = find_available_port()?;
        let cmd = CmdBuilder::ockam(&format!(
            "tcp-inlet create --at /node/{} --from 127.0.0.1:{inlet_port} --to /node/{}/service/outlet",
            node_2.name(),
            node_1.name()
        ))?;
        cmd.run()?.assert().success();

        let cmd = CmdBuilder::new(&format!("curl --fail --head 127.0.0.1:{inlet_port}"))?;
        cmd.run()?.assert().success();

        Ok(())
    }
}
