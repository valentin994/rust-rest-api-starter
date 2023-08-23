use axum::extract::{FromRef, FromRequestParts, RawQuery, State};
use axum::response::{ErrorResponse, Result};
use axum::{
    async_trait,
    http::request::Parts,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{de, Deserialize, Deserializer};
use std::net::SocketAddr;
use std::{fmt, str::FromStr};
mod models;

use models::users::{CreateUser, User};

use bb8::{Pool, PooledConnection};
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let manager = PostgresConnectionManager::new_from_stringlike(
        "host=localhost user=postgres password=mysecretpassword dbname=user_db",
        NoTls,
    )
    .unwrap();
    let pool = Pool::builder().build(manager).await.unwrap();
    let app = Router::new()
        .route("/", get(root))
        .route("/users", post(create_user))
        .route("/user", get(get_user))
        .with_state(pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

struct DatabaseConnection(PooledConnection<'static, PostgresConnectionManager<NoTls>>);

#[async_trait]
impl<S> FromRequestParts<S> for DatabaseConnection
where
    ConnectionPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = ConnectionPool::from_ref(state);

        let conn = pool.get_owned().await.map_err(internal_error)?;

        Ok(Self(conn))
    }
}

fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn create_user(
    State(pool): State<ConnectionPool>,
    Json(payload): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), ErrorResponse> {
    let connection = pool.get().await.map_err(internal_error)?;
    tracing::info!("{:?}", &payload.username);
    let row = connection
        .query(
            "INSERT INTO userTable(username) VALUES($1::TEXT) RETURNING id;",
            &[&payload.username],
        )
        .await
        .map_err(internal_error)?;
    let id: i32 = row[0].get("id");
    let user = User {
        id,
        username: payload.username,
    };
    Ok((StatusCode::CREATED, Json(user)))
}
#[derive(Debug, Deserialize)]
struct Params {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    username: Option<String>,
}

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(de::Error::custom).map(Some),
    }
}

async fn get_user(
    State(pool): State<ConnectionPool>,
    id: RawQuery,
) -> Result<(StatusCode, Json<User>), ErrorResponse> {
    let user = User {
        id: 2,
        username: format!("hello"),
    };
    tracing::info!("{:?}", id.0.unwrap());
    Ok((StatusCode::OK, Json(user)))
}
