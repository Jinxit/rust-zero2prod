use crate::domain::{NewSubscriber, SubscriberName};
use crate::models::NewSubscription;
use crate::startup::NewsletterDbConn;
use chrono::Utc;
use diesel::RunQueryDsl;
use rocket::form::Form;
use rocket::http::Status;
use uuid::Uuid;

#[derive(FromForm)]
pub struct FormData {
    name: String,
    email: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, conn),
    fields(
        request_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
#[post("/subscriptions", data = "<form>")]
pub async fn subscribe(form: Form<FormData>, conn: NewsletterDbConn) -> Result<(), Status> {
    let form = form.into_inner();
    let name = match SubscriberName::parse(form.name) {
        Ok(name) => name,
        Err(_) => return Err(Status::BadRequest),
    };
    let new_subscriber = NewSubscriber {
        email: form.email,
        name,
    };
    match insert_subscriber(new_subscriber, conn).await {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!("Failed to execute query: {:?}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, conn)
)]
async fn insert_subscriber(
    new_subscriber: NewSubscriber,
    conn: NewsletterDbConn,
) -> diesel::QueryResult<usize> {
    use crate::schema::subscriptions;
    conn.run(move |c| {
        diesel::insert_into(subscriptions::table)
            .values(NewSubscription {
                id: &Uuid::new_v4(),
                email: &new_subscriber.email,
                name: &new_subscriber.name.as_ref(),
                subscribed_at: &Utc::now(),
            })
            .execute(c)
    })
    .await
}
