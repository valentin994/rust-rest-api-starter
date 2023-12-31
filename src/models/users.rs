use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
}

#[derive(Deserialize)]
pub struct CreateUser {
    pub username: String,
}
