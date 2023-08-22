use axum::extract::{FromRef, FromRequestParts, State};
use axum::response::{ErrorResponse, Result};
use axum::{
    async_trait,
    http::request::Parts,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::any::Any;
use std::net::SocketAddr;

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
    // build our application with a route
    let app = Router::new()
        .route("/users", post(create_user))
        .route("/", get(root))
        .with_state(pool);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

async fn using_connection_pool_extractor(
    State(pool): State<ConnectionPool>,
    Json(payload): Json<CreateUser>,
) -> Result<String, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;
    let row = conn
        .query_one("select 1 + 1", &[])
        .await
        .map_err(internal_error)?;
    let two: i32 = row.try_get(0).map_err(internal_error)?;

    Ok(two.to_string())
}

// we can also write a custom extractor that grabs a connection from the pool
// which setup is appropriate depends on your application
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

async fn using_connection_extractor(
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<String, (StatusCode, String)> {
    println!("connected");
    let row = conn
        .query_one("select 1 + 1", &[])
        .await
        .map_err(internal_error)?;
    let two: i32 = row.try_get(0).map_err(internal_error)?;
    Ok(two.to_string())
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
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
