use crate::models::NewSubscription;
use crate::startup::NewsletterDbConn;
use chrono::Utc;
use diesel::RunQueryDsl;
use rocket::form::Form;
use rocket::http::Status;
use tracing::Instrument;
use uuid::Uuid;

#[derive(FromForm)]
pub struct FormData {
    name: String,
    email: String,
}

#[post("/subscriptions", data = "<form>")]
pub async fn subscribe(form: Form<FormData>, conn: NewsletterDbConn) -> Result<(), Status> {
    use crate::schema::subscriptions;
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name
    );
    let _request_span_guard = request_span.enter();

    let query_span = tracing::info_span!("Saving new subscriber details in the database");
    let result: diesel::QueryResult<usize> = conn
        .run(move |c| {
            diesel::insert_into(subscriptions::table)
                .values(NewSubscription {
                    id: &Uuid::new_v4(),
                    email: &form.email,
                    name: &form.name,
                    subscribed_at: &Utc::now(),
                })
                .execute(c)
        })
        .instrument(query_span)
        .await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!("Failed to execute query: {:?}", e);
            Err(Status::InternalServerError)
        }
    }
}
