//! Development-only HTTP adapter around the SAME native domain engine.
//! Not linked into the desktop app; publish only on host loopback via Compose.
use packflow_core::{DbWorker, DomainError, Mutation, Query};
use std::{env, io::Read, path::PathBuf};
use tiny_http::{Header, Method, Response, Server};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = PathBuf::from(env::var("PACKFLOW_DATA_DIR").unwrap_or_else(|_| ".dev-data".into()));
    let test_bypass = cfg!(debug_assertions)
        && root == std::path::Path::new("/test-data")
        && env::var("PACKFLOW_TEST_NO_AUTH").as_deref() == Ok("1");
    let mut account =
        packflow_core::preview_auth::PreviewAuth::open(&root.join("preview-account.sqlite"))?;
    let worker = DbWorker::start(root.join("packflow.sqlite"))?;
    let bind = env::var("PACKFLOW_PREVIEW_BIND").unwrap_or_else(|_| "127.0.0.1:4318".into());
    let server = Server::http(&bind)?;
    println!(
        "PackFlow development core listening on {bind}; schema {}, KWD.",
        packflow_core::db::SCHEMA_VERSION
    );
    for mut req in server.incoming_requests() {
        let mut set_cookie = None;
        let cookie = req
            .headers()
            .iter()
            .find(|h| h.field.equiv("Cookie"))
            .map(|h| h.value.as_str().to_string());
        let result = (|| -> Result<serde_json::Value, DomainError> {
            // No permissive CORS. Browser requests must use the local Vite proxy.
            if req.method() != &Method::Post {
                return Err(DomainError::validation("method", "POST required."));
            }
            if let Some(origin) = req.headers().iter().find(|h| h.field.equiv("Origin")) {
                if ![
                    "http://127.0.0.1:1420",
                    "http://localhost:1420",
                    "http://127.0.0.1:1421",
                    "http://localhost:1421",
                ]
                .contains(&origin.value.as_str())
                {
                    return Err(DomainError::new("FORBIDDEN", "Untrusted preview origin."));
                }
            }
            if req.body_length().unwrap_or(usize::MAX) > 1_048_576 {
                return Err(DomainError::validation("body", "Request too large."));
            }
            let mut body = String::new();
            std::io::Read::take(req.as_reader(), 1_048_577).read_to_string(&mut body)?;
            if body.len() > 1_048_576 {
                return Err(DomainError::validation("body", "Request too large."));
            }
            if req.url() == "/api/auth" {
                let request = serde_json::from_str::<packflow_core::preview_auth::Request>(&body)?;
                if test_bypass && matches!(request, packflow_core::preview_auth::Request::Status) {
                    return Ok(
                        serde_json::json!({"configured":true,"unlocked":true,"display_name":"Test fixture","test_bypass":true}),
                    );
                }
                let (value, cookie) = account.handle(request, cookie.as_deref())?;
                set_cookie = cookie;
                return Ok(value);
            }
            if !test_bypass {
                account.authorize(cookie.as_deref())?;
            }
            match req.url() {
                "/api/query" => worker.query(serde_json::from_str::<Query>(&body)?),
                "/api/mutate" => {
                    let mut request = serde_json::from_str::<Mutation>(&body)?;
                    if !test_bypass {
                        request.actor_id = packflow_core::models::MANAGER_ID.into();
                    }
                    worker.mutate(request)
                }
                _ => Err(DomainError::new(
                    "NOT_FOUND",
                    "Unknown development endpoint.",
                )),
            }
        })();
        let (status, body) = match result {
            Ok(value) => (200, value.to_string()),
            Err(error) => (
                if error.code == "UNAUTHENTICATED" {
                    401
                } else {
                    400
                },
                serde_json::to_string(&error)?,
            ),
        };
        let mut response = Response::from_string(body)
            .with_status_code(status)
            .with_header(Header::from_bytes("Content-Type", "application/json").unwrap());
        response.add_header(Header::from_bytes("Cache-Control", "no-store").unwrap());
        if let Some(cookie) = set_cookie {
            response.add_header(Header::from_bytes("Set-Cookie", cookie).unwrap());
        }
        let _ = req.respond(response);
    }
    Ok(())
}
