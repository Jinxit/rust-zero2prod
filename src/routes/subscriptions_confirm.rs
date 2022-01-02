use crate::models::SubscriptionToken;
use crate::startup::NewsletterDbConn;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};
use rocket::http::Status;
use uuid::Uuid;

#[tracing::instrument(name = "Confirm a pending subscriber", skip(subscription_token, conn))]
#[get("/subscriptions/confirm?<subscription_token>")]
pub async fn confirm(
    subscription_token: Option<&str>,
    conn: NewsletterDbConn,
) -> Result<(), Status> {
    let subscription_token = match subscription_token {
        Some(token) => token,
        None => return Err(Status::BadRequest),
    };
    let id = match get_subscriber_id_from_token(&conn, subscription_token.to_string()).await {
        Ok(id) => id,
        Err(_) => return Err(Status::InternalServerError),
    };
    match id {
        None => Err(Status::Unauthorized),
        Some(subscriber_id) => confirm_subscriber(&conn, subscriber_id)
            .await
            .map_err(|_| Status::InternalServerError),
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, conn))]
pub async fn confirm_subscriber(
    conn: &NewsletterDbConn,
    subscriber_id: Uuid,
) -> Result<(), diesel::result::Error> {
    use crate::schema::subscriptions::dsl::*;
    conn.run(move |c| {
        diesel::update(subscriptions.filter(id.eq(subscriber_id)))
            .set(status.eq("confirmed"))
            .execute(c)
            .map_err(|e| {
                tracing::error!("Failed to execute query: {:?}", e);
                e
            })
            .map(|_| ())
    })
    .await
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(token, conn))]
pub async fn get_subscriber_id_from_token(
    conn: &NewsletterDbConn,
    token: String,
) -> Result<Option<Uuid>, diesel::result::Error> {
    use crate::schema::subscription_tokens::dsl::*;
    conn.run(move |c| {
        subscription_tokens
            .filter(subscription_token.eq(token))
            .first::<SubscriptionToken>(c)
            .map(|st| st.subscriber_id)
            .optional()
            .map_err(|e| {
                tracing::error!("Failed to execute query: {:?}", e);
                e
            })
    })
    .await
}
