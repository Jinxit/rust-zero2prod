use rocket::http::Status;

#[tracing::instrument(name = "Confirm a pending subscriber", skip(subscription_token))]
#[get("/subscriptions/confirm?<subscription_token>")]
pub async fn confirm(subscription_token: Option<&str>) -> Result<(), Status> {
    let subscription_token = match subscription_token {
        Some(token) => token,
        None => return Err(Status::BadRequest),
    };
    Ok(())
}
