use crate::schema::subscriptions;
use chrono::offset::Utc;
use chrono::DateTime;

// TODO: can these be the same thing?
//       can Subscription have a lifetime?
//       or would that mean NewSubscription needs ownership?

#[derive(Queryable)]
pub struct Subscription {
    pub id: uuid::Uuid,
    pub email: String,
    pub name: String,
    pub subscribed_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Insertable)]
#[table_name = "subscriptions"]
pub struct NewSubscription<'a> {
    pub id: &'a uuid::Uuid,
    pub email: &'a str,
    pub name: &'a str,
    pub subscribed_at: &'a DateTime<Utc>,
    pub status: &'a str,
}
