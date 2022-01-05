use crate::domain::SubscriberEmail;
use crate::email::Email;
use crate::guards::BasicAuth;
use crate::routes::error_chain_fmt;
use crate::startup::NewsletterDbConn;
use anyhow::Context;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use rocket::http::Status;
use rocket::response::Responder;
use rocket::{Request, Response, State};
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[post("/newsletters", data = "<body>")]
pub async fn publish_newsletter(
    body: rocket::serde::json::Json<BodyData>,
    conn: NewsletterDbConn,
    email_client: &State<Arc<dyn Email>>,
    auth: BasicAuth,
) -> Result<(), PublishError> {
    let subscribers = conn
        .run(|conn: &mut PgConnection| get_confirmed_subscribers(conn))
        .await
        .context("Failed to fetch confirmed subscribers from database.")?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                );
            }
        }
    }
    Ok(())
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl<'r> Responder<'r, 'static> for PublishError {
    fn respond_to(self, _request: &'r Request<'_>) -> rocket::response::Result<'static> {
        tracing::warn!("PublishError: {:?}", self);
        Response::build()
            .status(match self {
                PublishError::UnexpectedError(_) => Status::InternalServerError,
            })
            .ok()
    }
}

pub struct ConfirmedSubscriber {
    pub email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(conn))]
fn get_confirmed_subscribers(
    conn: &PgConnection,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, diesel::result::Error> {
    use crate::schema::subscriptions as subs;
    let rows = subs::table
        .select(subs::email)
        .filter(subs::status.eq("confirmed"))
        .load::<String>(conn)?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|email| match SubscriberEmail::parse(email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();
    Ok(confirmed_subscribers)
}
