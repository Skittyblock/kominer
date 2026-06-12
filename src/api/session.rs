use crate::AppState;
use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "public_mode")]
pub struct DavSession {
    pub id: String,
    pub password: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct SessionRequest {
    id: String,
    password: String,
}

#[derive(Serialize)]
pub struct SessionResponse {
    id: String,
    username: String,
    password: String,
    expires_at: Option<DateTime<Utc>>,
}

pub async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<SessionRequest>,
) -> Result<Json<SessionResponse>, (StatusCode, String)> {
    #[cfg(feature = "public_mode")]
    if !is_valid_user_id(&req.id) {
        return Err((StatusCode::BAD_REQUEST, "user_id must be 16 digits".into()));
    }

    // see if there's an existing auth'd session
    #[cfg(feature = "public_mode")]
    {
        let sessions = state.sessions.read().await;
        let passes_auth = match sessions.get(&req.id) {
            Some(session) => session.id == req.id && session.password == req.password,
            None => state
                .auth
                .as_ref()
                .is_some_and(|auth| auth.user == req.id && auth.password == req.password),
        };
        if !passes_auth {
            return Err((
                StatusCode::UNAUTHORIZED,
                "password doesn't match active session".into(),
            ));
        }

        let is_new_session_id = !sessions.contains_key(&req.id);
        if is_new_session_id && sessions.len() >= crate::SESSION_LIMIT as usize {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                format!("too many people connected, try again later"),
            ));
        }
    }
    // ensure provided user and pass match private credentials
    #[cfg(not(feature = "public_mode"))]
    if !crate::validate_session(&state, &req.id, &req.password).await {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid username or password".into(),
        ));
    }

    let user_dir = state.storage_root.join(&req.id);
    tokio::fs::create_dir_all(&user_dir)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    #[cfg(feature = "public_mode")]
    let expires_at = Utc::now() + chrono::Duration::hours(1);

    #[cfg(feature = "public_mode")]
    {
        let session = DavSession {
            id: req.id.clone(),
            password: req.password.clone(),
            expires_at,
        };

        state.sessions.write().await.insert(req.id.clone(), session);
    }

    Ok(Json(SessionResponse {
        id: req.id.clone(),
        username: req.id,
        password: req.password,
        #[cfg(feature = "public_mode")]
        expires_at: Some(expires_at),
        #[cfg(not(feature = "public_mode"))]
        expires_at: None,
    }))
}

#[cfg(feature = "public_mode")]
fn is_valid_user_id(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| b.is_ascii_digit())
}
