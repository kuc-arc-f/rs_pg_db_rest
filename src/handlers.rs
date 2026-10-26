use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use sqlx::{postgres::PgRow, Column, Row, TypeInfo, PgPool, Postgres};
use sqlx::FromRow;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::{CreateTodoRequest, Todo};

#[derive(Debug,  Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
}
/// 1行分のデータを HashMap で保持
/// 値はすべて文字列化して格納（汎用取得のため）
pub type RowData = HashMap<String, Option<String>>;

pub async fn create_todo(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateTodoRequest>,
) -> Result<StatusCode, String> {
    let ret: Vec<Todo> = Vec::new();
    let title = payload.title.trim();
    if title.is_empty() {
        return Ok(StatusCode::BAD_REQUEST);
    }

    let todo = sqlx::query_as::<_, Todo>(
        r#"
        INSERT INTO todos (title)
        VALUES ($1)
        RETURNING id, title, completed, created_at
        "#,
    )
    .bind(title)
    .fetch_one(&pool)
    .await.unwrap();

    Ok(StatusCode::CREATED)
}

/// テーブルのカラム情報を取得
pub async fn get_columns(pool: &PgPool, table_name: &str) -> Result<Vec<ColumnInfo>> {
    // information_schema からカラム情報を取得
    // $1 は schema 名（public 固定）、$2 はテーブル名
    let rows = sqlx::query(
        r#"
        SELECT
            column_name,
            data_type,
            is_nullable
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = $1
        ORDER BY ordinal_position
        "#,
    )
    .bind(table_name)
    .fetch_all(pool)
    .await?;

    let cols = rows
        .into_iter()
        .map(|r| ColumnInfo {
            name: r.get::<String, _>("column_name"),
            data_type: r.get::<String, _>("data_type"),
            is_nullable: r.get::<String, _>("is_nullable") == "YES",
        })
        .collect();

    Ok(cols)
}

/// 指定テーブルから全データを取得（汎用）
/// 値は型情報を見て文字列に変換
pub async fn get_table_data(
    pool: &PgPool,
    table_name: &str,
    columns: &[ColumnInfo],
    sql: &str,
) -> Result<Vec<RowData>> {
    // テーブル名は SQL インジェクション対策のため識別子としてエスケープ
    // カラム名は information_schema から取得済みなので安全
    //let quoted_table = format!("\"{}\"", table_name.replace('"', "\"\""));

    let rows: Vec<PgRow> = sqlx::query(&sql).fetch_all(pool).await?;

    let mut result = Vec::with_capacity(rows.len());

    for row in &rows {
        let mut map: RowData = HashMap::new();

        for (idx, col) in row.columns().iter().enumerate() {
            let col_name = col.name().to_string();
            let type_name = col.type_info().name().to_uppercase();

            // NULL チェック
            let value: Option<String> = if let Ok(Some(v)) = try_get_as_string(row, idx, &type_name) {
                Some(v)
            } else {
                None
            };

            // 参考: `columns` 引数で列順を制御可能
            let _ = &columns;

            map.insert(col_name, value);
        }
        result.push(map);
    }

    Ok(result)
}


/// 型ごとに適切な変換を行って文字列化
fn try_get_as_string(row: &PgRow, idx: usize, type_name: &str) -> Result<Option<String>, sqlx::Error> {
    // 注意: NULL の場合は `try_get::<Option<T>, _>` で None が返る
    match type_name {
        "UUID" => {
            let v: Option<uuid::Uuid> = row.try_get(idx)?;
            Ok(v.map(|u| u.to_string()))
        }
        "TEXT" | "VARCHAR" | "NAME" | "BPCHAR" | "CHAR" => {
            let v: Option<String> = row.try_get(idx)?;
            Ok(v)
        }
        "BOOL" => {
            let v: Option<bool> = row.try_get(idx)?;
            Ok(v.map(|b| b.to_string()))
        }
        "INT2" => {
            let v: Option<i16> = row.try_get(idx)?;
            Ok(v.map(|n| n.to_string()))
        }
        "INT4" => {
            let v: Option<i32> = row.try_get(idx)?;
            Ok(v.map(|n| n.to_string()))
        }
        "INT8" => {
            let v: Option<i64> = row.try_get(idx)?;
            Ok(v.map(|n| n.to_string()))
        }
        "FLOAT4" => {
            let v: Option<f32> = row.try_get(idx)?;
            Ok(v.map(|n| n.to_string()))
        }
        "FLOAT8" => {
            let v: Option<f64> = row.try_get(idx)?;
            Ok(v.map(|n| n.to_string()))
        }
        "TIMESTAMPTZ" => {
            let v: Option<chrono::DateTime<chrono::Utc>> = row.try_get(idx)?;
            Ok(v.map(|d| d.to_rfc3339()))
        }
        "TIMESTAMP" => {
            let v: Option<chrono::NaiveDateTime> = row.try_get(idx)?;
            Ok(v.map(|d| d.to_string()))
        }
        "DATE" => {
            let v: Option<chrono::NaiveDate> = row.try_get(idx)?;
            Ok(v.map(|d| d.to_string()))
        }
        // 上記以外の型は TEXT としてフォールバック（要検証）
        _ => {
            let v: Option<String> = row.try_get(idx).ok().flatten();
            Ok(v)
        }
    }
}

/// まとめて取得するエントリーポイント
pub async fn inspect_table(
    pool: &PgPool,
    table_name: &str,
    sql: &str,
) -> Result<(Vec<ColumnInfo>, Vec<RowData>)> {
    let columns = get_columns(pool, table_name).await?;
    if columns.is_empty() {
        anyhow::bail!("table '{}' not found or has no columns", table_name);
    }
    let data = get_table_data(pool, table_name, &columns, sql).await?;
    Ok((columns, data))
}

pub async fn select_list(
  State(pool): State<PgPool>,
  Json(payload): Json<super::models::UpdataReq>,
) -> anyhow::Result<String, String>
{
    let table_name = &payload.table;
    let sql = &payload.sql;
    let schema_name = "public"; // デフォルトスキーマ

    //let (columns, data) = inspect_table(&pool, "todos").await.unwrap();
    let (columns, data) = inspect_table(&pool, table_name, sql).await.unwrap();
    println!("=== Columns ===");
    for c in &columns {
        println!("  {} : {} (nullable={})", c.name, c.data_type, c.is_nullable);
    }
    println!("\n=== Rows ({}) ===", data.len());
    //let out_items : Vec<super::models::SelectResp> = Vec::new();
    for (i, row) in data.iter().enumerate() {
        println!("--- row {} ---", i);
        for (k, v) in row {            
            println!("  {} = {:?}", k, v);
        }
    }    
    let resp_json = json!({
        "table_name": table_name,
        "data": data
    });    
    Ok(resp_json.to_string())
}


/// GET /todos : TODO一覧を取得する（登録日時の降順）
pub async fn list_todos(
  State(pool): State<PgPool>
) -> Json<Vec<Todo>> {
    let todos = sqlx::query_as::<_, Todo>(
        r#"
        SELECT id, title, completed, created_at
        FROM todos
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(&pool)
    .await.unwrap();

    Json(todos)
}

/// DELETE /todos/:id : 指定したIDのTODOを削除する
pub async fn delete_todo(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> StatusCode {
    let result = sqlx::query(
        r#"
        DELETE FROM todos WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(&pool)
    .await.unwrap();

    if result.rows_affected() == 0 {
        return StatusCode::BAD_REQUEST;
    }

    StatusCode::NO_CONTENT
}


pub async fn user_create(
    State(pool): State<PgPool>,
    Json(payload): Json<super::models::UserCreateReq>,
) -> Result<StatusCode, String>
{
    let mut user = super::models::UserRes {
        email: "".to_string(),
        password: "".to_string(),
    };
    let sql = "
        SELECT id , email FROM users WHERE email = $1;
    ";
    let rows = sqlx::query(sql)
        .bind(&payload.email)
        .fetch_all(&pool)
        .await.unwrap();
    let mut user_ct = 0;
    for row in rows {
        user_ct += 1;
    }
    println!("user_ct={}", user_ct);
    if user_ct > 0 {
        println!("error , email NG");
        return Ok(StatusCode::BAD_REQUEST);
    }
    let pw = payload.password.clone();
    let phc = super::mod_auth::hash_password(&pw).expect("error , hash_password");
    println!("phc: {}", phc);   
    let ok = super::mod_auth::verify_password(&pw, &phc).expect("error , veryfy password");
    println!("password ok? {}", ok);    
    let result = sqlx::query(
        "INSERT INTO users (email , password_hash) VALUES ($1, $2)"
    )
    .bind(&payload.email)
    .bind(&phc)
    .execute(&pool)
    .await.unwrap();
    
    Ok(StatusCode::CREATED)
}

pub async fn user_login(
    State(pool): State<PgPool>,
    Json(payload): Json<super::models::UserCreateReq>,
) -> Result<StatusCode, String> {
    #[derive(Debug, FromRow)]
    struct User {
        id: i32,
        email: String,
        password_hash: String,
        created_at: chrono::DateTime<chrono::Utc>,
    }      
    println!("&payload.email={}", &payload.email);  
    println!("&payload.password={}", &payload.password);  
    let user = super::models::UserRes {
        email: "".to_string(),
        password: "".to_string(),
    };
    let sql = "
        SELECT id , email, password_hash FROM users WHERE email = $1;
    ";
    let rows = sqlx::query(sql)
        .bind(&payload.email)
        .fetch_all(&pool)
        .await.unwrap();
    let mut user_ct = 0;
    for row in rows {
        user_ct += 1;
    }
    println!("user_ct={}", user_ct);
    if user_ct == 0 {
        println!("error , email NG");
        return Ok(StatusCode::BAD_REQUEST);
    }
    let pw = payload.password.clone();

    // SELECT
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT
            id,
            email,
            password_hash,
            created_at
        FROM users
        WHERE email = $1
        "#
    )
    .bind(&payload.email)
    .fetch_optional(&pool)
    .await.unwrap();  
    let mut phc : String = "" .to_string();
    match user {
        Some(user) => {
            println!("id: {}", user.id);
            println!("email: {}", user.email);
            println!("password_hash: {}", user.password_hash);
            phc = user.password_hash.clone();
            println!("created_at: {}", user.created_at);
        }
        None => {
            println!("データが見つかりません");
        }
    }   
    let ok = super::mod_auth::verify_password(&pw, &phc).expect("error , veryfy password");
    println!("password ok? {}", ok);    
    if ok == false {
        println!("error , password NG");
        return Ok(StatusCode::BAD_REQUEST);
    }       

    Ok(StatusCode::OK)
}