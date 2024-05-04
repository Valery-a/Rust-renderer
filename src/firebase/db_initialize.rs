
use std::{env, sync::Arc};
use firebase_rs::*;
use crate::firebase::db_operations::{User, set_user, get_users, get_user, update_user, delete_user};
extern crate dotenv;
use dotenv::dotenv;

pub async fn db_start(){
    dotenv().ok();
    let user = User {
        name: "mazna".to_string(),
        age: 1337,
        email: "megamazna".to_string(),
    };
    let firebase_key = env
        ::var("FIREBASE_SECRET_KEY")
        .expect("Expected a secret key in the environment");
    let firebase = Firebase::new(&firebase_key).unwrap();

    let response = set_user(&firebase, &user).await;

    let users = get_users(&firebase).await;
    println!("{:?}", users);
    let mut user = get_user(&firebase, &response.name).await;
    println!("{:?}", user);

    user.email = "updated.mail@gmail.com".to_string();
    let updated_user = update_user(&firebase, &response.name, &user).await;
    println!("{:?}", updated_user);

    //delete_user(&firebase, &response.name).await;
    //println!("User deleted");
}

pub fn initialize_firebase() -> Arc<Firebase> {
    dotenv().ok(); // Load environment variables
    let firebase_key = env::var("FIREBASE_SECRET_KEY")
                        .expect("FIREBASE_SECRET_KEY must be set");
    let firebase = Firebase::new(&firebase_key).expect("Failed to initialize Firebase");
    Arc::new(firebase)
}