use crate::domain::SubscriberName;
use crate::domain::{NewSubscriber, SubscriberEmail};
use crate::email::Email;
use crate::models::NewSubscription;
use crate::startup::NewsletterDbConn;
use chrono::Utc;
use diesel::RunQueryDsl;
use rocket::form::Form;
use rocket::http::Status;
use rocket::State;
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
    skip(form, conn, email_client),
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
) -> Result<(), Status> {
    let new_subscriber = match form.into_inner().try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return Err(Status::BadRequest),
    };
    if insert_subscriber(&new_subscriber, conn).await.is_err() {
        return Err(Status::InternalServerError);
    }

    if send_confirmation_email(email_client, new_subscriber)
        .await
        .is_err()
    {
        return Err(Status::InternalServerError);
    }
    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber)
)]
async fn send_confirmation_email(
    email_client: &State<Arc<dyn Email>>,
    new_subscriber: NewSubscriber,
) -> anyhow::Result<()> {
    let confirmation_link = "https://my-api.com/subscriptions/confirm";
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
async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    conn: NewsletterDbConn,
) -> diesel::QueryResult<usize> {
    use crate::schema::subscriptions;
    let email = new_subscriber.email.as_ref().to_string();
    let name = new_subscriber.name.as_ref().to_string();
    conn.run(move |c| {
        diesel::insert_into(subscriptions::table)
            .values(NewSubscription {
                id: &Uuid::new_v4(),
                email: &email,
                name: &name,
                subscribed_at: &Utc::now(),
                status: "confirmed",
            })
            .execute(c)
    })
    .await
}
