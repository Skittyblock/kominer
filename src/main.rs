use axum::{
    Router,
    http::{HeaderMap, header::AUTHORIZATION},
    response::Redirect,
    routing::{any, get, post},
};
use base64::Engine;
use dav_server::{DavHandler, localfs::LocalFs};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::{RwLock, broadcast};
use tower_http::services::ServeDir;

mod api;
mod dav;
mod web;

#[cfg(feature = "public_mode")]
mod cleanup;

use api::events::FileEvent;

pub const PUBLIC_MODE: bool = cfg!(feature = "public_mode");
pub const DB_FILE_NAME: &str = "vocabulary_builder.sqlite3";
pub const SESSION_LIMIT: u32 = 20;

#[derive(Clone)]
pub struct AppState {
    storage_root: PathBuf,
    dav_handler: DavHandler,
    #[cfg(not(feature = "public_mode"))]
    auth: Arc<PrivateAuth>,
    #[cfg(feature = "public_mode")]
    auth: Option<Arc<PrivateAuth>>,
    #[cfg(feature = "public_mode")]
    sessions: Arc<RwLock<HashMap<String, api::session::DavSession>>>,
    notifiers: Arc<RwLock<HashMap<String, broadcast::Sender<FileEvent>>>>,
}

#[derive(Clone)]
pub struct PrivateAuth {
    user: String,
    password: String,
}

#[tokio::main]
async fn main() {
    #[cfg(debug_assertions)]
    dotenv::dotenv().ok();

    let storage_root = std::env::temp_dir().join("kominer-webdav");

    let dav_handler = DavHandler::builder()
        .filesystem(LocalFs::new(storage_root.clone(), false, false, false))
        .locksystem(dav_server::memls::MemLs::new())
        .build_handler();

    #[cfg(not(feature = "public_mode"))]
    let auth = Arc::new(PrivateAuth {
        user: std::env::var("KOMINER_USER")
            .expect("KOMINER_USER env var is required in non-public mode"),
        password: std::env::var("KOMINER_PASSWORD")
            .expect("KOMINER_PASSWORD env var is required in non-public mode"),
    });
    #[cfg(feature = "public_mode")]
    let auth = match (
        std::env::var("KOMINER_USER"),
        std::env::var("KOMINER_PASSWORD"),
    ) {
        (Ok(user), Ok(password)) => Some(Arc::new(PrivateAuth { user, password })),
        _ => None,
    };

    let state = AppState {
        storage_root,
        dav_handler,
        #[cfg(not(feature = "public_mode"))]
        auth,
        #[cfg(feature = "public_mode")]
        auth,
        #[cfg(feature = "public_mode")]
        sessions: Arc::new(RwLock::new(HashMap::new())),
        notifiers: Arc::new(RwLock::new(HashMap::new())),
    };

    println!("storage location: {}", state.storage_root.display());

    // clear old session files
    #[cfg(feature = "public_mode")]
    {
        tokio::fs::remove_dir_all(&state.storage_root)
            .await
            .unwrap();
        if let Some(ref auth) = state.auth {
            let user_dir = state.storage_root.join(&auth.user);
            tokio::fs::create_dir_all(&user_dir).await.unwrap();
        }
    };

    tokio::fs::create_dir_all(&state.storage_root)
        .await
        .unwrap();

    #[cfg(feature = "public_mode")]
    tokio::spawn(cleanup::clean_sessions(state.clone()));

    let app = Router::new()
        .route("/", get(web::index))
        .nest_service("/static", ServeDir::new("static"))
        .route("/api/events", get(api::events))
        .route("/api/session", post(api::create_session))
        .route("/api/vocabulary", get(api::get_vocabulary))
        .route("/dav", any(|| async { Redirect::permanent("/dav/") }))
        .route("/dav/", any(dav::handle))
        .route("/dav/{*path}", any(dav::handle))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or("3000".into());
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    println!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

pub async fn validate_session(state: &AppState, username: &str, password: &str) -> bool {
    #[cfg(feature = "public_mode")]
    {
        let sessions = state.sessions.read().await;
        match sessions.get(username) {
            Some(session) => {
                session.id == username
                    && session.password == password
                    && session.expires_at > chrono::Utc::now()
            }
            None => state
                .auth
                .as_ref()
                .is_some_and(|auth| auth.user == username && auth.password == password),
        }
    }
    #[cfg(not(feature = "public_mode"))]
    {
        return username == state.auth.user && password == state.auth.password;
    }
}

pub fn parse_basic_auth(headers: &HeaderMap) -> Option<(String, String)> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let encoded = value.strip_prefix("Basic ")?;

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;

    let decoded = String::from_utf8(decoded).ok()?;
    let (username, password) = decoded.split_once(':')?;

    Some((username.to_string(), password.to_string()))
}
