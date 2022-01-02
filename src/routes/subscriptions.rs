use crate::domain::SubscriberName;
use crate::domain::{NewSubscriber, SubscriberEmail};
use crate::email::Email;
use crate::models::{NewSubscription, NewSubscriptionToken};
use crate::schema::subscription_tokens;
use crate::startup::{ApplicationBaseUrl, NewsletterDbConn};
use chrono::Utc;
use diesel::{PgConnection, RunQueryDsl};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rocket::form::Form;
use rocket::http::Status;
use rocket::State;
use std::borrow::Borrow;
use std::sync::Arc;
use uuid::Uuid;

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
) -> Result<(), Status> {
    let new_subscriber = match form.into_inner().try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return Err(Status::BadRequest),
    };
    let (subscription_token, new_subscriber) = conn
        .run_transaction::<_, diesel::result::Error, _>(move |conn| {
            let subscriber_id = insert_subscriber(&new_subscriber, conn)?;
            let subscription_token = generate_subscription_token();
            store_token(conn, &subscriber_id, &subscription_token)?;
            Ok((subscription_token, new_subscriber))
        })
        .await
        .map_err(|_| Status::InternalServerError)?;

    if send_confirmation_email(
        email_client.borrow(),
        new_subscriber,
        &base_url.inner().0,
        &subscription_token,
    )
    .await
    .is_err()
    {
        return Err(Status::InternalServerError);
    }
    Ok(())
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, conn)
)]
pub fn store_token(
    conn: &PgConnection,
    subscriber_id: &Uuid,
    subscription_token: &str,
) -> Result<(), diesel::result::Error> {
    diesel::insert_into(subscription_tokens::table)
        .values(NewSubscriptionToken {
            subscription_token,
            subscriber_id,
        })
        .execute(conn)
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })
        .map(|_| ())
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
) -> anyhow::Result<()> {
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
        .send_email(new_subscriber.email, "Welcome!", html_body, plain_body)
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
        .execute(conn)
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })
        .map(|_| subscriber_id)
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
