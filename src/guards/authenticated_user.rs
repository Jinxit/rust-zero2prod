use crate::guards::{BasicAuth, OrStatus};
use crate::models::User;
use crate::startup::NewsletterDbConn;
use crate::telemetry::spawn_blocking_with_tracing;
use anyhow::anyhow;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use diesel::OptionalExtension;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use rocket::http::Status;
use rocket::outcome::try_outcome;
use rocket::outcome::Outcome::{Failure, Success};
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use secrecy::{ExposeSecret, Secret};
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
            anyhow!("User did not supply Basic Auth credentials.")
        )));

        match validate_credentials(conn, basic_auth).await {
            Ok(user) => Success(user),
            Err((status, err)) => Failure((status, err)),
        }
    }
}

#[tracing::instrument(name = "Validate credentials", skip(conn, basic_auth))]
async fn validate_credentials(
    conn: NewsletterDbConn,
    basic_auth: BasicAuth,
) -> Result<AuthenticatedUser, (Status, anyhow::Error)> {
    let user: Option<User> = get_stored_credentials(conn, basic_auth.username).await?;

    let user = user.or_status(Status::Unauthorized, "Unknown username.")?;
    let expected_password_hash = Secret::new(user.password_hash);

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, basic_auth.password)
    })
    .await
    .or_status(Status::InternalServerError, "Failed to spawn/join thread.")??;

    Ok(AuthenticatedUser {
        user_id: user.user_id,
        username: user.username,
        _private: (),
    })
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), (Status, anyhow::Error)> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .or_status(
            Status::InternalServerError,
            "Failed to parse hash in PHC string format.",
        )?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .or_status(Status::Unauthorized, "Invalid password.")
}

#[tracing::instrument(name = "Get stored credentials", skip(conn, username))]
async fn get_stored_credentials(
    conn: NewsletterDbConn,
    username: String,
) -> Result<Option<User>, (Status, anyhow::Error)> {
    conn.run(move |conn: &mut PgConnection| {
        use crate::schema::users;

        users::table
            .filter(users::username.eq(username))
            .first::<User>(conn)
            .optional()
    })
    .await
    .or_status(
        Status::InternalServerError,
        "Failed to perform a query to retrieve stored credentials.",
    )
}
