use axum::extract::{Query, State};
use axum::response::{ErrorResponse, Result};
use axum::{
    http::StatusCode,
    routing::{get, patch, post},
    Json, Router,
};
use sqlx::Error::RowNotFound;
use std::collections::HashMap;
use std::net::SocketAddr;

mod models;

use models::users::{CreateUser, User};

use sqlx::postgres::PgPool;
use sqlx::{Pool, Postgres, Row};

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let url = "postgres://postgres:mysecretpassword@localhost:5432/user_db";
    let pool = PgPool::connect(url).await.unwrap();
    let app = Router::new()
        .route("/", get(root))
        .route("/user", post(create_user))
        .route("/user", get(get_user))
        .route("/user", patch(update_user))
        .with_state(pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn create_user(
    State(pool): State<Pool<Postgres>>,
    Json(payload): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), ErrorResponse> {
    tracing::info!("{:?}", &payload.username);
    let query = "INSERT INTO userTable(username) VALUES($1::TEXT) RETURNING id;";
    let res = sqlx::query(query)
        .bind(&payload.username)
        .fetch_one(&pool)
        .await
        .map_err(internal_error)?;
    let id: i32 = res.get("id");
    let user = User {
        id,
        username: payload.username,
    };
    Ok((StatusCode::CREATED, Json(user)))
}

async fn update_user(
    State(pool): State<Pool<Postgres>>,
    Json(payload): Json<User>,
) -> Result<(StatusCode, Json<User>), ErrorResponse> {
    tracing::info!("{:?}", &payload.id);
    let query = format!(
        "UPDATE userTable SET username = '{}' where id={} RETURNING *;",
        &payload.username, &payload.id
    );
    let q = sqlx::query_as::<_, User>(&query);
    let user = q.fetch_one(&pool).await.map_err(internal_error)?;
    Ok((StatusCode::CREATED, Json(user)))
}

fn internal_error(err: sqlx::Error) -> (StatusCode, String) {
    tracing::info!(
        "Server encontered an error while communicating with the database {:?}",
        err.to_string()
    );
    match err {
        RowNotFound => (StatusCode::NOT_FOUND, String::from("Can't find user")),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            String::from("fucking hell"),
        ),
    }
}

async fn get_user(
    State(pool): State<Pool<Postgres>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<User>), ErrorResponse> {
    tracing::info!("Get user with = {:?}", params);
    let username = params.get("username").unwrap();
    let q = format!(
        "SELECT id, username FROM userTable WHERE username='{}'",
        &username
    );
    let query = sqlx::query_as::<_, User>(&q).bind(&username);
    let user = query.fetch_one(&pool).await.map_err(internal_error)?;
    Ok((StatusCode::OK, Json(user)))
}
