use crate::spinner::Spinner;
use crate::AppError;
use comfy_table::Table;
use ockam::{route, Context, TcpTransport, TCP};

pub struct InletCommand {}

impl InletCommand {
    pub async fn run(
        ctx: &Context,
        inlet_host_port: &str,
        outlet_host_port: &str,
        outlet_name: &str,
    ) -> Result<(), AppError> {
        let spinner = Spinner::default();

        let tcp = TcpTransport::create(ctx).await?;
        let route_to_outlet = route![(TCP, outlet_host_port), outlet_name];

        tcp.create_inlet(inlet_host_port, route_to_outlet).await?;

        spinner.stop("Created inlet");

        let mut table = Table::new();
        table
            .set_header(vec!["Inlet", "Listener", "Destination"])
            .add_row(vec![
                inlet_host_port,
                format!("{} ({})", outlet_host_port, outlet_name).as_str(),
            ]);

        println!("{}", table);

        Ok(())
    }
}
