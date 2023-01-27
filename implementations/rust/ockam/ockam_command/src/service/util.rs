use core::fmt::Write;
use ockam_api::nodes::models::services::ServiceList;

use crate::util::output::Output;

impl Output for ServiceList<'_> {
    fn output(&self) -> anyhow::Result<String> {
        if self.list.is_empty() {
            return Ok("No services found".to_string());
        }

        let mut w = String::new();
        write!(w, "Services:")?;

        let services_list = self.list.clone();
        for service in services_list {
            write!(w, "\n  Service: ")?;
            write!(w, "\n    Type: {}", service.service_type)?;
            write!(w, "\n    Address: /service/{}", service.addr)?;
        }

        Ok(w)
    }
}
