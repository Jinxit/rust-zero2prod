use crate::models::NewSubscription;
use crate::startup::NewsletterDbConn;
use chrono::Utc;
use diesel::RunQueryDsl;
use rocket::form::Form;
use uuid::Uuid;

#[derive(FromForm)]
pub struct FormData {
    name: String,
    email: String,
}

#[post("/subscriptions", data = "<form>")]
pub async fn subscribe(form: Form<FormData>, conn: NewsletterDbConn) {
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
            .unwrap(); // TODO: panic triggers 500, but performs poorly
    })
    .await;
}
