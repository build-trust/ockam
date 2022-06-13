use std::io::{self, Write};

use clap::Args;
use reqwest::StatusCode;
use serde_json::json;
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;
use validator::validate_email;

use ockam_api::error::ApiError;

use crate::enroll::EnrollCommand;
use crate::old::identity::load_or_create_identity;
use crate::util::embedded_node;
use crate::IdentityOpts;

const API_SECRET: &str = "DNYsEfhe]ms]ET]yQIthmhSOIvCkWOnb";

#[derive(Clone, Debug, Args)]
pub(crate) struct EnrollEmailCommand;

impl EnrollEmailCommand {
    pub fn run(command: EnrollCommand) {
        println!("\nThank you for trying Ockam. We are working towards a developer release of Ockam Orchestrator in September.
Please tell us your email and we'll let you know when we're ready to enroll new users to Ockam Orchestrator.\n");
        let email = read_user_input().expect("couldn't read user input");
        embedded_node(enroll, (command, email));
    }
}

fn read_user_input() -> anyhow::Result<String> {
    let mut buffer = String::new();
    loop {
        print!("Email: ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut buffer)?;
        let email = buffer.trim();
        if validate_email(email) {
            return Ok(email.to_string());
        } else {
            println!("\nThe email address is not valid, try again.\n");
            buffer.clear();
        }
    }
}

async fn enroll(mut ctx: ockam::Context, args: (EnrollCommand, String)) -> anyhow::Result<()> {
    let (command, email) = args;

    let _identity = load_or_create_identity(&IdentityOpts::from(&command), &ctx).await?;

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
            println!("Enrolled successfully");
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
