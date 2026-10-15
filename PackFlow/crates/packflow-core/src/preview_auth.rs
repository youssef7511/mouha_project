//! Authentication for the loopback development preview. Never linked into desktop builds.
use crate::{
    auth::LocalAuth,
    error::{DomainError, Result},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Status,
    Setup {
        username: String,
        display_name: String,
        password: String,
    },
    Login {
        username: String,
        password: String,
        remember: bool,
    },
    Logout,
    ChangePassword {
        current: String,
        password: String,
    },
    Recover {
        code: String,
        password: String,
    },
}
pub struct PreviewAuth {
    account: LocalAuth,
}
impl PreviewAuth {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            account: LocalAuth::open(path)?,
        })
    }
    pub fn authorize(&mut self, cookie: Option<&str>) -> Result<()> {
        let token = cookie
            .and_then(|c| {
                c.split(';')
                    .find_map(|part| part.trim().strip_prefix("packflow_preview="))
            })
            .ok_or_else(|| {
                DomainError::new("UNAUTHENTICATED", "Sign in to access this workspace.")
            })?;
        self.account.resume(token)
    }
    pub fn handle(
        &mut self,
        request: Request,
        cookie: Option<&str>,
    ) -> Result<(Value, Option<String>)> {
        let clear =
            "packflow_preview=; HttpOnly; SameSite=Strict; Path=/api; Max-Age=0".to_string();
        let mut set_cookie = None;
        let mut recovery = None;
        let mut unlocked = false;
        match request {
            Request::Status => {
                unlocked = self.authorize(cookie).is_ok();
            }
            Request::Setup {
                username,
                display_name,
                password,
            } => {
                recovery = Some(self.account.setup(&username, &display_name, &password)?);
            }
            Request::Login {
                username,
                password,
                remember,
            } => {
                self.account.login(&username, &password)?;
                let token = self.account.remember()?;
                set_cookie = Some(format!(
                    "packflow_preview={token}; HttpOnly; SameSite=Strict; Path=/api{}",
                    if remember { "; Max-Age=2592000" } else { "" }
                ));
                unlocked = true;
            }
            Request::Logout => {
                self.authorize(cookie)?;
                self.account.logout()?;
                set_cookie = Some(clear);
            }
            Request::ChangePassword { current, password } => {
                self.authorize(cookie)?;
                self.account.change_password(&current, &password)?;
                set_cookie = Some(clear);
            }
            Request::Recover { code, password } => {
                recovery = Some(self.account.recover(&code, &password)?);
                set_cookie = Some(clear);
            }
        }
        let mut status = self.account.status()?;
        status["unlocked"] = json!(unlocked);
        if !unlocked {
            status["display_name"] = json!("");
        }
        status["recovery_code"] = json!(recovery);
        Ok((status, set_cookie))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_requires_cookie_revokes_logout_and_resumes_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("preview-account.sqlite");
        let mut a = PreviewAuth::open(&path).unwrap();
        assert!(a.authorize(None).is_err());
        a.handle(
            Request::Setup {
                username: "owner".into(),
                display_name: "Owner".into(),
                password: "Preview password 123".into(),
            },
            None,
        )
        .unwrap();
        let (status, cookie) = a
            .handle(
                Request::Login {
                    username: "owner".into(),
                    password: "Preview password 123".into(),
                    remember: true,
                },
                None,
            )
            .unwrap();
        assert_eq!(status["unlocked"], true);
        let cookie = cookie.unwrap();
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Max-Age=2592000"));
        assert!(a.authorize(None).is_err());
        assert_eq!(
            a.handle(Request::Status, None).unwrap().0["unlocked"],
            false
        );
        a.authorize(Some(&cookie)).unwrap();
        drop(a);
        let mut a = PreviewAuth::open(&path).unwrap();
        a.authorize(Some(&cookie)).unwrap();
        a.handle(Request::Logout, Some(&cookie)).unwrap();
        assert!(a.authorize(Some(&cookie)).is_err());
        let (_, cookie) = a
            .handle(
                Request::Login {
                    username: "owner".into(),
                    password: "Preview password 123".into(),
                    remember: false,
                },
                None,
            )
            .unwrap();
        assert!(!cookie.unwrap().contains("Max-Age"));
    }
}
