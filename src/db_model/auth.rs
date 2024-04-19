use bcrypt::{hash, verify};
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use crate::models::User;
use std::env;

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, bcrypt::DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
}

pub fn generate_token(user: &User) -> String {
    let secret_key = env::var("SECRET_KEY").expect("SECRET_KEY must be set");
    let token = encode(&Header::default(), user, &EncodingKey::from_secret(secret_key.as_ref())).unwrap();
    token
}

pub fn decode_token(token: &str) -> Result<User, jsonwebtoken::errors::Error> {
    let secret_key = env::var("SECRET_KEY").expect("SECRET_KEY must be set");
    let token_data = decode::<User>(token, &DecodingKey::from_secret(secret_key.as_ref()), &Validation::default())?;
    Ok(token_data.claims)
}
