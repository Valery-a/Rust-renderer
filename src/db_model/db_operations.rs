use diesel::prelude::*;
use crate::db::establish_connection;
use crate::models::{User, NewUser};

pub fn create_user(name: &str, email: &str) -> usize {
    use crate::schema::users::dsl::*;

    let conn = establish_connection();
    let new_user = NewUser { name, email };

    diesel::insert_into(users)
        .values(&new_user)
        .execute(&conn)
        .expect("Error saving new user")
}

pub fn read_users() -> Vec<User> {
    use crate::schema::users::dsl::*;

    let conn = establish_connection();
    users.load::<User>(&conn)
        .expect("Error loading users")
}

pub fn update_user(user_id: i32, new_name: &str) -> usize {
    use crate::schema::users::dsl::*;

    let conn = establish_connection();
    diesel::update(users.find(user_id))
        .set(name.eq(new_name))
        .execute(&conn)
        .expect("Error updating user")
}

pub fn delete_user(user_id: i32, current_user: &User) -> Result<usize, AppError> {
    if current_user.role != "admin" {
        return Err(AppError::Unauthorized);
    }

    use crate::schema::users::dsl::*;
    let conn = establish_connection();
    diesel::delete(users.find(user_id))
        .execute(&conn)
        .map_err(Into::into)
}


pub fn update_user_email_and_name(user_id: i32, new_email: &str, new_name: &str) -> Result<usize, AppError> {
    use crate::schema::users::dsl::*;
    let conn = establish_connection();
    conn.transaction::<_, AppError, _>(|| {
        diesel::update(users.find(user_id)).set(name.eq(new_name)).execute(&conn)?;
        diesel::update(users.find(user_id)).set(email.eq(new_email)).execute(&conn)?;
        Ok(())
    })
}

pub fn create_user(name: &str, email: &str) -> Result<usize, AppError> {
    if name.is_empty() || email.is_empty() {
        return Err(AppError::InvalidInput("Name and email cannot be empty".into()));
    }

    use crate::schema::users::dsl::*;
    let conn = establish_connection();
    let new_user = NewUser { name, email };

    let result = diesel::insert_into(users)
        .values(&new_user)
        .execute(&conn)?;
    Ok(result)
}
