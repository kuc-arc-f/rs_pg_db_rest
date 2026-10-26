use anyhow::{ensure, Context, Result};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, PasswordHash, rand_core::OsRng};
use std::error::Error;

/// パスワードをハッシュ化して PHC 文字列を返す（DB にこの文字列を保存）
pub fn hash_password(password: &str) -> anyhow::Result<String, String> {
    let argon2 = Argon2::default();

    // ランダムソルト生成
    let salt = SaltString::generate(&mut OsRng);

    // ハッシュを生成して PHC 形式の文字列にする（例: $argon2id$v=19$m=4096,t=3,p=1$...）
    let password_hash = argon2.hash_password(password.as_bytes(), &salt)
    .expect("error , hash_password");

    Ok(password_hash.to_string())
}

/// 入力パスワードが保存されたハッシュ（PHC 文字列）と一致するか検証
pub fn verify_password(password: &str, stored_phc: &str) -> anyhow::Result<bool, String> 
{
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(stored_phc).expect("error , PasswordHash::new2);");
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false), // パスワード不一致
        Err(e) => Err("error, argon2.verify_password".to_string()), // その他エラー
    }
}

