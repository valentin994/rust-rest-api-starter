use std::any::Any;
use std::fmt::Error;
use axum::{
    async_trait,
    routing::{get, post},
    http::request::Parts,
    http::StatusCode,
    response::IntoResponse,
    Json,
    Router,

};
use axum::body::Body;
use axum::extract::{State, Path, Query, FromRequestParts, FromRef};
use axum::response::Result;
use std::net::SocketAddr;

use serde_json::{json, Value};
mod models;
use models::users::{User, CreateUser};

use bb8::{Pool, PooledConnection};
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let manager =
        PostgresConnectionManager::new_from_stringlike("host=localhost user=postgres password=mysecretpassword", NoTls)
            .unwrap();
    let pool = Pool::builder().build(manager).await.unwrap();
    // build our application with a route
    let app = Router::new()
        .route("/users", post(using_connection_pool_extractor))
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
) -> Result<String, (StatusCode, String)> {
    tracing::debug!("tried to connect");
    let conn = pool.get().await.map_err(internal_error)?;
    tracing::debug!("connected");
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

async fn create_user(payload: Json<User>) -> Result<Json<Value>, Error > {
    Ok(Json(json!(*payload)))
}
