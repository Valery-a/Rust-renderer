use super::schema::users;
use diesel::Queryable;

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub role: String,
}

#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub email: &'a str,
}
