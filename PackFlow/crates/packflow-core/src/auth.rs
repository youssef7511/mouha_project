//! Device-local account, kept outside portable business backups.
use crate::{
    db,
    error::{DomainError, Result},
};
use argon2::{
    password_hash::{PasswordHash, SaltString},
    Argon2, PasswordHasher, PasswordVerifier,
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::path::Path;

pub struct LocalAuth {
    conn: Connection,
    unlocked: bool,
}
fn secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
fn password_hash(password: &str) -> Result<String> {
    if password.chars().count() < 10 || password.len() > 1024 {
        return Err(DomainError::validation(
            "password",
            "Use a password of at least 10 characters (maximum 1024 bytes).",
        ));
    }
    let salt = SaltString::encode_b64(uuid::Uuid::new_v4().as_bytes())
        .map_err(|_| DomainError::storage("Could not generate account salt."))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| DomainError::storage("Could not protect password."))
}
impl LocalAuth {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS account(id INTEGER PRIMARY KEY CHECK(id=1), username TEXT NOT NULL, display_name TEXT NOT NULL, password_hash TEXT NOT NULL, recovery_hash TEXT NOT NULL, remember_hash TEXT, failed INTEGER NOT NULL DEFAULT 0, retry_after INTEGER NOT NULL DEFAULT 0) STRICT;")?;
        Ok(Self {
            conn,
            unlocked: false,
        })
    }
    pub fn exists(&self) -> Result<bool> {
        Ok(self
            .conn
            .query_row("SELECT EXISTS(SELECT 1 FROM account)", [], |r| r.get(0))?)
    }
    pub fn status(&self) -> Result<Value> {
        Ok(
            json!({"configured":self.exists()?,"unlocked":self.unlocked,"display_name":if self.unlocked { self.conn.query_row("SELECT display_name FROM account WHERE id=1",[],|r|r.get::<_,String>(0))? } else {String::new()}}),
        )
    }
    pub fn require(&self) -> Result<()> {
        if self.unlocked {
            Ok(())
        } else {
            Err(DomainError::new(
                "UNAUTHENTICATED",
                "Sign in to access this workspace.",
            ))
        }
    }
    pub fn setup(&mut self, username: &str, name: &str, password: &str) -> Result<String> {
        if self.exists()? {
            return Err(DomainError::new(
                "CONFLICT",
                "An owner account already exists.",
            ));
        }
        let username = crate::service::text(username, 100, "username", true)?;
        let name = crate::service::text(name, 100, "display_name", true)?;
        let hash = password_hash(password)?;
        let recovery = secret();
        self.conn.execute("INSERT INTO account(id,username,display_name,password_hash,recovery_hash) VALUES(1,?1,?2,?3,?4)",params![username,name,hash,db::hash(recovery.as_bytes())])?;
        // Setup does not unlock: user must save the recovery code then sign in.
        Ok(recovery)
    }
    fn attempt(&self) -> Result<()> {
        let retry: i64 =
            self.conn
                .query_row("SELECT retry_after FROM account WHERE id=1", [], |r| {
                    r.get(0)
                })?;
        if chrono::Utc::now().timestamp() < retry {
            return Err(DomainError::new(
                "RATE_LIMITED",
                "Too many attempts. Try again in a minute.",
            ));
        }
        Ok(())
    }
    fn failed(&self) -> Result<()> {
        self.conn.execute("UPDATE account SET failed=failed+1,retry_after=CASE WHEN failed>=4 THEN ?1+60 ELSE 0 END WHERE id=1",[chrono::Utc::now().timestamp()])?;
        Err(DomainError::new(
            "UNAUTHENTICATED",
            "Credentials were not accepted.",
        ))
    }
    pub fn login(&mut self, username: &str, password: &str) -> Result<()> {
        self.attempt()?;
        let (user, hash): (String, String) = self.conn.query_row(
            "SELECT username,password_hash FROM account WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let valid = password.len() <= 1024
            && PasswordHash::new(&hash).ok().is_some_and(|h| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &h)
                    .is_ok()
            });
        if !valid || username.trim() != user {
            return self.failed();
        }
        self.conn.execute(
            "UPDATE account SET failed=0,retry_after=0,remember_hash=NULL WHERE id=1",
            [],
        )?;
        self.unlocked = true;
        Ok(())
    }
    pub fn remember(&self) -> Result<String> {
        self.require()?;
        let token = secret();
        self.conn.execute(
            "UPDATE account SET remember_hash=?1 WHERE id=1",
            [db::hash(token.as_bytes())],
        )?;
        Ok(token)
    }
    pub fn resume(&mut self, token: &str) -> Result<()> {
        if token.len() != 64 {
            return Err(DomainError::new("UNAUTHENTICATED", "Sign in again."));
        }
        let valid: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM account WHERE remember_hash=?1)",
            [db::hash(token.as_bytes())],
            |r| r.get(0),
        )?;
        if !valid {
            return Err(DomainError::new("UNAUTHENTICATED", "Sign in again."));
        }
        self.unlocked = true;
        Ok(())
    }
    pub fn logout(&mut self) -> Result<()> {
        self.unlocked = false;
        self.conn
            .execute("UPDATE account SET remember_hash=NULL WHERE id=1", [])?;
        Ok(())
    }
    pub fn change_password(&mut self, current: &str, password: &str) -> Result<()> {
        self.require()?;
        let user: String =
            self.conn
                .query_row("SELECT username FROM account WHERE id=1", [], |r| r.get(0))?;
        self.login(&user, current)?;
        let hash = password_hash(password)?;
        self.conn.execute(
            "UPDATE account SET password_hash=?1,remember_hash=NULL WHERE id=1",
            [hash],
        )?;
        self.logout()
    }
    pub fn recover(&mut self, code: &str, password: &str) -> Result<String> {
        self.attempt()?;
        let expected: String =
            self.conn
                .query_row("SELECT recovery_hash FROM account WHERE id=1", [], |r| {
                    r.get(0)
                })?;
        if code.trim().len() != 64 || db::hash(code.trim().as_bytes()) != expected {
            self.failed()?;
        }
        let hash = password_hash(password)?;
        let recovery = secret();
        self.conn.execute("UPDATE account SET password_hash=?1,recovery_hash=?2,remember_hash=NULL,failed=0,retry_after=0 WHERE id=1",params![hash,db::hash(recovery.as_bytes())])?;
        self.unlocked = false;
        Ok(recovery)
    }
}
