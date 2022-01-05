use anyhow::{anyhow, Context};
use rocket::http::Status;
use rocket::outcome::Outcome::{Failure, Success};
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use secrecy::Secret;

pub struct BasicAuth {
    username: String,
    password: Secret<String>,
}

#[async_trait]
impl<'r> FromRequest<'r> for BasicAuth {
    type Error = anyhow::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match from_request_result(request) {
            Ok(auth) => Success(auth),
            Err(e) => Failure((Status::Unauthorized, e)),
        }
    }
}

fn from_request_result(request: &Request) -> Result<BasicAuth, anyhow::Error> {
    let header_value = request
        .headers()
        .get_one("Authorization")
        .context("The 'Authorization' header was missing")?;

    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;

    let decoded_bytes = base64::decode_config(base64encoded_segment, base64::STANDARD)
        .context("Failed to base64-decode 'Basic' credentials.")?;

    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    let mut credentials = decoded_credentials.splitn(2, ':');

    let username = credentials
        .next()
        .ok_or_else(|| anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();

    let password = credentials
        .next()
        .ok_or_else(|| anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();

    Ok(BasicAuth {
        username,
        password: Secret::new(password),
    })
}
