use axum::{
    extract::{Path, State},
    Json,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

mod handlers;
mod models;
mod mod_auth;

use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // .env ファイルの読み込み（存在すれば）
    dotenvy::dotenv().ok();

    // ロガー初期化
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "todo_api=debug,tower_http=debug".into()),
        )
        .init();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL 環境変数が設定されていません");

    // コネクションプール作成
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await?;

    // ルーティング定義
    let app = Router::new()
        .route("/todos", post(handlers::create_todo).get(handlers::list_todos))
        .route("/todos/:id", axum::routing::delete(handlers::delete_todo))
        .route("/api/update", post(update))
        .route("/api/select", post(handlers::select_list))
        .route("/api/signup", post(handlers::user_create))
        .route("/api/login", post(handlers::user_login))
        .route("/health", get(|| async { "ok" }))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8888));
    println!("#Start-localhost:8888");
    tracing::info!("サーバーを起動しました: http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub async fn update(
    State(pool): State<PgPool>,
    Json(payload): Json<models::UpdataReq>,
) -> Result<StatusCode, String> {
    let sql = payload.sql.trim();
    if sql.is_empty() {
        println!("sql={}", &sql);
        return Ok(StatusCode::BAD_REQUEST);
    }
    let todo = sqlx::query(sql)
    .execute(&pool)
    .await.unwrap();

    Ok(StatusCode::CREATED)
}