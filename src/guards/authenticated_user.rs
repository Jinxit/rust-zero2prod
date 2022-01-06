use crate::guards::BasicAuth;
use crate::startup::NewsletterDbConn;
use anyhow::{anyhow, Context};
use diesel::OptionalExtension;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use rocket::http::Status;
use rocket::outcome::{try_outcome, IntoOutcome};
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use secrecy::ExposeSecret;
use uuid::Uuid;

pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub username: String,
    // prevents construction outside of this module
    _private: (),
}

#[async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = anyhow::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let conn = try_outcome!(request.guard::<NewsletterDbConn>().await.map_failure(|_| (
            Status::InternalServerError,
            anyhow!("Failed to retrieve a connection from the DB pool.")
        )));
        let basic_auth = try_outcome!(request.guard::<BasicAuth>().await.map_failure(|_| (
            Status::Unauthorized,
            anyhow!("User has not been authenticated.")
        )));

        from_request_result(basic_auth, conn)
            .await
            .into_outcome(Status::Unauthorized)
    }
}

async fn from_request_result(
    basic_auth: BasicAuth,
    conn: NewsletterDbConn,
) -> Result<AuthenticatedUser, anyhow::Error> {
    conn.run(move |conn: &mut PgConnection| {
        use crate::schema::users;

        let user_id = users::table
            .select(users::user_id)
            .filter(users::username.eq(&basic_auth.username))
            .filter(users::password.eq(basic_auth.password.expose_secret()))
            .first::<Uuid>(conn)
            .optional()
            .context("Failed to perform a query to validate auth credentials.")?
            .ok_or_else(|| anyhow!("Invalid username or password."))?;

        Ok(AuthenticatedUser {
            user_id,
            username: basic_auth.username,
            _private: (),
        })
    })
    .await
}
