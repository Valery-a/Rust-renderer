use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

use diesel::result::{DatabaseErrorKind, Error as DieselError};
use std::convert::From;
use std::error::Error;
use std::fmt;


pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

#[derive(Debug)]
pub enum AppError {
    DatabaseError(String),
    NotFound,
    InvalidInput(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AppError::DatabaseError(ref err) => write!(f, "Database error: {}", err),
            AppError::NotFound => write!(f, "Resource not found"),
            AppError::InvalidInput(ref err) => write!(f, "Invalid input: {}", err),
        }
    }
}

impl Error for AppError {}

impl From<DieselError> for AppError {
    fn from(error: DieselError) -> AppError {
        match error {
            DieselError::NotFound => AppError::NotFound,
            DieselError::DatabaseError(kind, info) => {
                let message = match kind {
                    DatabaseErrorKind::UniqueViolation => "Unique constraint violation",
                    _ => "Database error",
                };
                AppError::DatabaseError(format!("{}: {}", message, info.message()))
            },
            _ => AppError::DatabaseError("Unhandled database error".into()),
        }
    }
}
