use miette::IntoDiagnostic;
use ockam_api::journeys::{JourneyEvent, APPLICATION_EVENT_COMMAND};
use ockam_api::CliState;
use ockam_core::OCKAM_TRACER_NAME;
use ockam_node::Executor;
use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::{global, Context};
use std::collections::HashMap;
use tracing::error;

/// This function creates a journey event describing the execution of a command
pub fn add_command_event(
    cli_state: CliState,
    command: &str,
    command_arguments: String,
) -> miette::Result<()> {
    let command_name = command.to_string();
    let tracer = global::tracer(OCKAM_TRACER_NAME);
    tracer
        .in_span(command_name.clone(), |_| {
            Executor::execute_future(async move {
                let mut attributes = HashMap::new();
                attributes.insert(APPLICATION_EVENT_COMMAND, command_arguments);
                cli_state
                    .add_journey_event(JourneyEvent::ok(command_name), attributes)
                    .await
            })
        })
        .into_diagnostic()??;
    Ok(())
}

/// This function creates a journey event describing the error resulting from the execution of a command
pub fn add_command_error_event(
    cli_state: CliState,
    command_name: &str,
    message: &str,
    command_arguments: String,
) -> miette::Result<()> {
    let message = message.to_string();
    let command = command_name.to_string();
    let tracer = global::tracer(OCKAM_TRACER_NAME);
    tracer
        .in_span(format!("'{}' error", command), |_| {
            Context::current()
                .span()
                .set_status(opentelemetry::trace::Status::error(message.clone()));
            error!("{}", &message);

            Executor::execute_future(async move {
                let mut attributes = HashMap::new();
                attributes.insert(APPLICATION_EVENT_COMMAND, command_arguments);
                cli_state
                    .add_journey_error(&command, message, attributes)
                    .await
            })
        })
        .into_diagnostic()??;
    Ok(())
}
