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
    match insert_subscriber(form.into_inner(), conn).await {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!("Failed to execute query: {:?}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, conn)
)]
async fn insert_subscriber(form: FormData, conn: NewsletterDbConn) -> diesel::QueryResult<usize> {
    use crate::schema::subscriptions;
    conn.run(move |c| {
        diesel::insert_into(subscriptions::table)
            .values(NewSubscription {
                id: &Uuid::new_v4(),
                email: &form.email,
                name: &form.name,
                subscribed_at: &Utc::now(),
            })
            .execute(c)
    })
    .await
}
