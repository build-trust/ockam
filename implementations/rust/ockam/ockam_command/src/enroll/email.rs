use clap::Args;
use reqwest::StatusCode;
use serde_json::json;
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;
use validator::validate_email;

use ockam_api::error::ApiError;

use crate::util::embedded_node;
use crate::{CommandGlobalOpts, EnrollCommand};

const API_SECRET: &str = "DNYsEfhe]ms]ET]yQIthmhSOIvCkWOnb";

#[derive(Clone, Debug, Args)]
pub struct EnrollEmailCommand;

impl EnrollEmailCommand {
    pub fn run(_opts: CommandGlobalOpts, cmd: EnrollCommand) {
        println!("\nThank you for trying Ockam. We are working towards a developer release of Ockam Orchestrator in September.
Please tell us your email and we'll let you know when we're ready to enroll new users to Ockam Orchestrator.\n");
        let email = read_user_input().expect("couldn't read user input");
        if let Err(e) = embedded_node(enroll, (cmd, email)) {
            eprintln!("Ockam node failed: {:?}", e,);
        }
    }
}

fn read_user_input() -> anyhow::Result<String> {
    loop {
        let email: String = dialoguer::Input::new()
            .with_prompt("Email")
            .interact_text()?;
        if validate_email(&email) {
            return Ok(email);
        } else {
            println!("\nThe email address is not valid, try again.\n");
        }
    }
}

async fn enroll(
    mut ctx: ockam::Context,
    (_cmd, email): (EnrollCommand, String),
) -> anyhow::Result<()> {
    let retry_strategy = ExponentialBackoff::from_millis(10).take(5);
    let res = Retry::spawn(retry_strategy, move || {
        let client = reqwest::Client::new();
        client
            .post("https://hub.ockam.network/api/enroll_email")
            .header("content-type", "application/json")
            .bearer_auth(API_SECRET)
            .body(json!({ "email": email }).to_string())
            .send()
    })
    .await
    .map_err(|err| ApiError::generic(&err.to_string()))?;
    match res.status() {
        StatusCode::NO_CONTENT => {
            println!("Thank you.");
            ctx.stop().await?;
            Ok(())
        }
        _ => {
            let res = res
                .text()
                .await
                .map_err(|err| ApiError::generic(&err.to_string()))?;
            let msg = "couldn't enroll using email";
            tracing::error!("{msg} [response={res:#?}]");
            Err(ApiError::generic(msg).into())
        }
    }
}
