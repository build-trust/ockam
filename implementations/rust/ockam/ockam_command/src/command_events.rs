use ockam_api::cli_state::journeys::{JourneyEvent, APPLICATION_EVENT_COMMAND};
use ockam_api::CliState;
use ockam_core::OCKAM_TRACER_NAME;
use opentelemetry::trace::{FutureExt, Span, TraceContextExt, Tracer};
use opentelemetry::{global, Context};
use std::collections::HashMap;
use tracing::warn;

/// This function creates a journey event describing the execution of a command
pub async fn add_command_event(
    cli_state: CliState,
    command: &str,
    command_arguments: String,
) -> miette::Result<()> {
    let command_name = command.to_string();
    let tracer = global::tracer(OCKAM_TRACER_NAME);

    let span = tracer.start(command_name.clone());
    let ctx = Context::current_with_span(span);

    let mut attributes = HashMap::new();
    attributes.insert(
        APPLICATION_EVENT_COMMAND,
        sanitize_command_arguments(command_arguments),
    );
    if let Err(e) = cli_state
        .add_journey_event(JourneyEvent::ok(command_name), attributes)
        .with_context(ctx)
        .await
    {
        warn!("cannot save a journey event: {}", e);
    }

    Ok(())
}

/// This function creates a journey event describing the error resulting from the execution of a command
pub async fn add_command_error_event(
    cli_state: CliState,
    command_name: &str,
    message: &str,
    command_arguments: String,
) -> miette::Result<()> {
    let message = message.to_string();
    let command = command_name.to_string();
    let tracer = global::tracer(OCKAM_TRACER_NAME);
    let mut span = tracer.start(format!("'{}' error", command));
    span.set_status(opentelemetry::trace::Status::error(message.clone()));
    let ctx = Context::current_with_span(span);

    let mut attributes = HashMap::new();
    attributes.insert(
        APPLICATION_EVENT_COMMAND,
        sanitize_command_arguments(command_arguments),
    );
    if let Err(e) = cli_state
        .add_journey_error(&command, message, attributes)
        .with_context(ctx)
        .await
    {
        warn!("cannot save a journey event: {}", e);
    }

    Ok(())
}

/// The ockam project enroll command arguments contain the enrollment ticket which is sensitive
/// information (because it could be potentially reused), so it should be removed from the user event.
pub fn sanitize_command_arguments(command_args: String) -> String {
    if command_args.starts_with("ockam project enroll") {
        "ockam project enroll".to_string()
    } else {
        command_args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_project_enroll() {
        assert_eq!(
            sanitize_command_arguments("ockam project enroll abcdxyz".to_string()),
            "ockam project enroll".to_string()
        );
        assert_eq!(
            sanitize_command_arguments("ockam node create n1".to_string()),
            "ockam node create n1".to_string()
        );
    }
}
