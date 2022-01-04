use crate::domain::SubscriberName;
use crate::domain::{NewSubscriber, SubscriberEmail};
use crate::email::Email;
use crate::models::{NewSubscription, NewSubscriptionToken};
use crate::routes::error_chain_fmt;
use crate::startup::{ApplicationBaseUrl, NewsletterDbConn};
use anyhow::Context;
use chrono::Utc;
use diesel::{PgConnection, RunQueryDsl};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rocket::form::Form;
use rocket::http::Status;
use rocket::response::Responder;
use rocket::{Request, Response, State};
use std::borrow::Borrow;
use std::error::Error;
use std::fmt::Formatter;
use std::sync::Arc;
use uuid::Uuid;

#[tracing::instrument(
name = "Adding a new subscriber",
skip(form, conn, email_client, base_url),
    fields(
        request_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
#[post("/subscriptions", data = "<form>")]
pub async fn subscribe(
    form: Form<FormData>,
    conn: NewsletterDbConn,
    email_client: &State<Arc<dyn Email>>,
    base_url: &State<ApplicationBaseUrl>,
) -> Result<(), SubscribeError> {
    let new_subscriber = form
        .into_inner()
        .try_into()
        .map_err(SubscribeError::ValidationError)?;
    let (subscription_token, new_subscriber) = conn
        .run_transaction::<_, SubscribeError, _, _>(
            move |conn| {
                let subscriber_id = insert_subscriber(&new_subscriber, conn)
                    .context("Failed to insert new subscriber in the database.")?;
                let subscription_token = generate_subscription_token();
                store_token(conn, &subscriber_id, &subscription_token)
                    .context("Failed to store the confirmation token for a new subscriber.")?;
                Ok((subscription_token, new_subscriber))
            },
            |e| {
                anyhow::Error::new(e)
                    .context("Failed to commit SQL transaction to store a new subscriber.")
                    .into()
            },
        )
        .await?;

    send_confirmation_email(
        email_client.borrow(),
        new_subscriber,
        &base_url.inner().0,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email.")?;
    Ok(())
}

#[derive(FromForm)]
pub struct FormData {
    name: String,
    email: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let email = SubscriberEmail::parse(form.email)?;
        let name = SubscriberName::parse(form.name)?;
        Ok(NewSubscriber { email, name })
    }
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl<'r> Responder<'r, 'static> for SubscribeError {
    fn respond_to(self, _request: &'r Request<'_>) -> rocket::response::Result<'static> {
        tracing::warn!("SubscribeError: {:?}", self);
        Response::build()
            .status(match self {
                SubscribeError::ValidationError(_) => Status::BadRequest,
                SubscribeError::UnexpectedError(_) => Status::InternalServerError,
            })
            .ok()
    }
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, conn)
)]
pub fn store_token(
    conn: &PgConnection,
    subscriber_id: &Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    use crate::schema::subscription_tokens::dsl::subscription_tokens;
    diesel::insert_into(subscription_tokens)
        .values(NewSubscriptionToken {
            subscription_token,
            subscriber_id,
        })
        .execute(conn)
        .map_err(StoreTokenError)?;
    Ok(())
}

pub struct StoreTokenError(diesel::result::Error);

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
             trying to store a subscription token."
        )
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url)
)]
async fn send_confirmation_email(
    email_client: &Arc<dyn Email>,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), anyhow::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let html_body = &format!(
        "Welcome to our newsletter!<br />\
                Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    let plain_body = &format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(&new_subscriber.email, "Welcome!", html_body, plain_body)
        .await
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, conn)
)]
fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    conn: &PgConnection,
) -> Result<Uuid, diesel::result::Error> {
    use crate::schema::subscriptions;
    let subscriber_id = Uuid::new_v4();
    diesel::insert_into(subscriptions::table)
        .values(NewSubscription {
            id: &subscriber_id,
            email: new_subscriber.email.as_ref(),
            name: new_subscriber.name.as_ref(),
            subscribed_at: &Utc::now(),
            status: "pending_confirmation",
        })
        .execute(conn)?;
    Ok(subscriber_id)
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
