use crate::schema::subscription_tokens;

#[derive(Queryable)]
pub struct SubscriptionToken {
    pub subscription_token: String,
    pub subscriber_id: uuid::Uuid,
}

#[derive(Insertable)]
#[table_name = "subscription_tokens"]
pub struct NewSubscriptionToken<'a> {
    pub subscription_token: &'a str,
    pub subscriber_id: &'a uuid::Uuid,
}
