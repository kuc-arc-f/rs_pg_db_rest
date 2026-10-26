use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdataReq {
    pub table: String,
    pub sql: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SelectResp {
    pub key: String,
    pub value: String,
}
/// DBから取得するTodoの構造体
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Todo {
    pub id: Uuid,
    pub title: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
}

/// TODO登録時のリクエストボディ
#[derive(Debug, Deserialize)]
pub struct CreateTodoRequest {
    pub title: String,
}

/// エラーレスポンス用
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserRes {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UserCreateReq {
    pub email: String,
    pub password: String,
}