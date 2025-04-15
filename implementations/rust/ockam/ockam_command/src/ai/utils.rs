use crate::CommandGlobalOpts;
use ockam_api::colors::color_primary;
use ockam_api::fmt_log;

pub async fn get_customer_name(
    opts: &CommandGlobalOpts,
    customer: Option<&str>,
) -> miette::Result<String> {
    match customer {
        Some(customer) => Ok(customer.to_string()),
        None => {
            let customer = opts
                .state
                .get_default_user()
                .await?
                .email
                .domain()?
                .replace('.', "-");
            opts.terminal.write_line(fmt_log!(
                "Retrieved customer {} from enrolled user data\n",
                color_primary(&customer),
            ))?;
            Ok(customer)
        }
    }
}
