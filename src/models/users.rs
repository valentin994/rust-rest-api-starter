use serde::{Serialize, Deserialize};

#[derive(Serialize)]
pub struct User {
    pub id: i32,
    pub username: String,
}

#[derive(Deserialize)]
pub struct CreateUser {
    pub username: String,
}
