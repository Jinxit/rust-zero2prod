mod authenticated_user;
mod basic_auth;

use anyhow::{anyhow, Context};
pub use authenticated_user::*;
pub use basic_auth::*;
use rocket::http::Status;

trait OrStatus<T> {
    fn or_status(self, status: Status, context: &'static str)
        -> Result<T, (Status, anyhow::Error)>;
}

impl<T, E> OrStatus<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn or_status(
        self,
        status: Status,
        context: &'static str,
    ) -> Result<T, (Status, anyhow::Error)> {
        self.context(context).map_err(|e| (status, e))
    }
}

impl<T> OrStatus<T> for Option<T> {
    fn or_status(
        self,
        status: Status,
        context: &'static str,
    ) -> Result<T, (Status, anyhow::Error)> {
        self.ok_or_else(|| (status, anyhow!(context)))
    }
}
