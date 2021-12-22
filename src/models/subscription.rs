#[derive(Queryable)]
pub struct Subscription {
    pub id: uuid::Uuid,
    pub email: String,
    pub name: String,
    pub subscribed_at: chrono::NaiveDateTime,
}
