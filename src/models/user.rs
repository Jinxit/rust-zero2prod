use crate::schema::users;

#[derive(Queryable)]
pub struct User {
    pub username: String,
    pub password: String,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub user_id: &'a uuid::Uuid,
    pub username: &'a str,
    pub password: &'a str,
}
