use base64::Engine as _;
use packflow_mobile_core::{
    error::{DomainError, Result},
    models::{Mutation, Query},
    Engine,
};
use std::{env, io::Read, path::PathBuf};
use tiny_http::{Header, Method, Response};
fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root =
        PathBuf::from(env::var("PACKFLOW_MOBILE_DATA").unwrap_or_else(|_| ".dev-data".into()));
    let mut core = Engine::open(&root.join("mobile.sqlite"), &root.join("backups"))?;
    let bind = env::var("PACKFLOW_MOBILE_BIND").unwrap_or_else(|_| "127.0.0.1:4328".into());
    let server = tiny_http::Server::http(&bind)?;
    println!("PackFlow Mobile preview on {bind}; standalone device data only.");
    for mut req in server.incoming_requests() {
        let mut download = None;
        let result = (|| -> Result<Vec<u8>> {
            if req.method() != &Method::Post {
                return Err(DomainError::validation("method", "POST required."));
            }
            if let Some(origin) = req.headers().iter().find(|h| h.field.equiv("Origin")) {
                if ![
                    "http://127.0.0.1:1430",
                    "http://localhost:1430",
                    "http://127.0.0.1:1431",
                    "http://localhost:1431",
                ]
                .contains(&origin.value.as_str())
                {
                    return Err(DomainError::new("FORBIDDEN", "Untrusted preview origin."));
                }
            }
            const LIMIT: u64 = 36 * 1024 * 1024;
            if req.body_length().is_some_and(|n| n as u64 > LIMIT) {
                return Err(DomainError::validation("body", "Request too large."));
            }
            let mut body = String::new();
            req.as_reader().take(LIMIT + 1).read_to_string(&mut body)?;
            if body.len() as u64 > LIMIT {
                return Err(DomainError::validation("body", "Request too large."));
            }
            let value = match req.url() {
                "/api/query" => core.query(serde_json::from_str::<Query>(&body)?)?,
                "/api/mutate" => core.mutate(serde_json::from_str::<Mutation>(&body)?)?,
                "/api/backup" => {
                    let request: serde_json::Value = serde_json::from_str(&body)?;
                    let name = request["name"].as_str();
                    let bytes = if let Some(name) = name {
                        core.read_backup(name)?
                    } else {
                        core.backup_bytes()?
                    };
                    download = Some(name.map(str::to_owned).unwrap_or_else(|| {
                        format!("PackFlow-Mobile-{}.pfmobile", uuid::Uuid::new_v4())
                    }));
                    return Ok(bytes);
                }
                "/api/restore" => {
                    let v: serde_json::Value = serde_json::from_str(&body)?;
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(v["data"].as_str().unwrap_or(""))
                        .map_err(|_| DomainError::validation("file", "Invalid backup encoding."))?;
                    core.restore(&bytes, v["confirmation"].as_str().unwrap_or(""))?
                }
                _ => {
                    return Err(DomainError::new(
                        "NOT_FOUND",
                        "Unknown mobile preview endpoint.",
                    ))
                }
            };
            Ok(serde_json::to_vec(&value)?)
        })();
        let (status, bytes) = match result {
            Ok(bytes) => (200, bytes),
            Err(e) => (400, serde_json::to_vec(&e)?),
        };
        let mut response = Response::from_data(bytes)
            .with_status_code(status)
            .with_header(Header::from_bytes("Cache-Control", "no-store").unwrap())
            .with_header(
                Header::from_bytes(
                    "Content-Type",
                    if download.is_some() {
                        "application/octet-stream"
                    } else {
                        "application/json"
                    },
                )
                .unwrap(),
            );
        if let Some(name) = download {
            response.add_header(
                Header::from_bytes(
                    "Content-Disposition",
                    format!("attachment; filename=\"{name}\""),
                )
                .unwrap(),
            );
        }
        let _ = req.respond(response);
    }
    Ok(())
}
