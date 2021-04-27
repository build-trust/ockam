use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use webbrowser;

const GITHUB_CLIENT_ID: &'static str = "put_client_id_here";

pub async fn post(
    client: &reqwest::Client,
    url: &str,
    payload: &HashMap<&str, &str>,
) -> Result<HashMap<String, String>, reqwest::Error> {
    let response = client.post(url).json(&payload).send().await?.text().await?;

    let parsed_response =
        url::form_urlencoded::parse(response.as_bytes()).fold(HashMap::new(), |mut acc, s| {
            acc.insert(s.0.to_string(), s.1.to_string());
            acc
        });

    Ok(parsed_response)
}

pub fn try_access_token<'a>(
    client: &'a reqwest::Client,
    payload: &'a HashMap<&str, &str>,
    time_to_retry: u64,
) -> Pin<Box<dyn Future<Output = Result<HashMap<String, String>, reqwest::Error>> + 'a>> {
    Box::pin(async move {
        let response = post(
            client,
            "https://github.com/login/oauth/access_token",
            payload,
        )
        .await?;

        if let Some(error) = response.get("error") {
            if error == "authorization_pending" {
                let duration = std::time::Duration::from_secs(time_to_retry);
                std::thread::sleep(duration);
                return try_access_token(client, payload, time_to_retry).await;
            }
        }

        println!("Access token validated!");
        return Ok(response);
    })
}

pub async fn authenticate() -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();

    let mut login_request = HashMap::new();
    login_request.insert("client_id", GITHUB_CLIENT_ID);

    let login_response = post(
        &client,
        "https://github.com/login/device/code",
        &login_request,
    )
    .await?;

    println!(
        "Put this code on the browser: {}",
        login_response["user_code"]
    );

    println!(
        "
-----------------------------------------------
        "
    );

    let duration = std::time::Duration::from_secs(2);
    std::thread::sleep(duration);

    let verification_uri = &login_response["verification_uri"];
    let interval = &login_response["interval"];
    let device_code = &login_response["device_code"];

    if !webbrowser::open(verification_uri).is_ok() {
        println!("Error opening the browser");
    }

    let mut access_token_request = HashMap::new();
    access_token_request.insert("client_id", GITHUB_CLIENT_ID);
    access_token_request.insert("device_code", device_code);
    access_token_request.insert("grant_type", "urn:ietf:params:oauth:grant-type:device_code");

    println!("Waiting to enter with code in browser...");

    try_access_token(
        &client,
        &access_token_request,
        interval.parse::<u64>().unwrap(),
    )
    .await?;

    println!("github authentication!");

    Ok(())
}
