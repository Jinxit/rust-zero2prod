use crate::schema::users;
use uuid::Uuid;

#[derive(Queryable)]
pub struct User {
    pub user_id: Uuid,
    pub username: String,
    pub password_hash: String,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub user_id: &'a uuid::Uuid,
    pub username: &'a str,
    pub password_hash: &'a str,
}
