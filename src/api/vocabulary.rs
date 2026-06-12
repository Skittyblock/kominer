use crate::{AppState, DB_FILE_NAME, parse_basic_auth, validate_session};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Serialize;
use tokio_rusqlite::OpenFlags;

#[derive(Serialize)]
pub struct VocabularyResponse {
    items: Vec<VocabularyItem>,
}

#[derive(Serialize)]
struct VocabularyItem {
    word: String,
    title_id: i64,
    create_time: i64,
    prev_context: Option<String>,
    next_context: Option<String>,
    highlight: Option<String>,
}

pub async fn get_vocabulary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<VocabularyResponse>, (StatusCode, String)> {
    let Some((username, password)) = parse_basic_auth(&headers) else {
        return Err((StatusCode::UNAUTHORIZED, "missing authorization".into()));
    };

    if !validate_session(&state, &username, &password).await {
        return Err((StatusCode::UNAUTHORIZED, "invalid session".into()));
    }

    let db_path = state.storage_root.join(&username).join(DB_FILE_NAME);

    let exists = tokio::fs::try_exists(&db_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !exists {
        return Err((StatusCode::NOT_FOUND, "database not found".into()));
    }

    let conn = tokio_rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = conn
        .call(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT word, title_id, create_time, prev_context, next_context, highlight
                FROM vocabulary
                ORDER BY create_time DESC
                "#,
            )?;

            let rows = stmt.query_map([], |row| {
                Ok(VocabularyItem {
                    word: row.get(0)?,
                    title_id: row.get(1)?,
                    create_time: row.get(2)?,
                    prev_context: row.get(3)?,
                    next_context: row.get(4)?,
                    highlight: row.get(5)?,
                })
            })?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }

            Ok::<_, tokio_rusqlite::rusqlite::Error>(items)
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(VocabularyResponse { items }))
}
