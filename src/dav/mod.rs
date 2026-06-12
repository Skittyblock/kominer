use crate::{AppState, DB_FILE_NAME, api, parse_basic_auth, validate_session};
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Request, StatusCode, Uri, header::WWW_AUTHENTICATE},
    response::Response,
};

const MAX_DB_SIZE: u64 = 25 * 1024 * 1024; // 25mb

pub async fn handle(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut req: Request<Body>,
) -> Response<Body> {
    let Some((username, password)) = parse_basic_auth(&headers) else {
        println!("header missing");
        return unauthorized();
    };

    if !validate_session(&state, &username, &password).await {
        return unauthorized();
    }

    if !is_allowed_method(req.method()) {
        return Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::from("method not allowed"))
            .unwrap();
    }

    if !is_allowed_dav_path(req.uri().path()) {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from(format!("only {DB_FILE_NAME} is allowed")))
            .unwrap();
    }

    if req.method() == Method::PUT {
        let Some(length) = content_length(&headers) else {
            return Response::builder()
                .status(StatusCode::LENGTH_REQUIRED)
                .body(Body::from("Content-Length required for uploads"))
                .unwrap();
        };

        if length > MAX_DB_SIZE {
            return Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .body(Body::from(format!(
                    "file too large; max {} bytes",
                    MAX_DB_SIZE
                )))
                .unwrap();
        }

        if req.uri().path() != format!("/dav/{DB_FILE_NAME}") {
            return Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body(Body::from(format!(
                    "uploads are only allowed to {DB_FILE_NAME}",
                )))
                .unwrap();
        }
    }

    let is_upload = req.method() == Method::PUT;

    let new_uri = match rewrite_dav_uri(req.uri(), &username) {
        Ok(uri) => uri,
        Err(msg) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(msg))
                .unwrap();
        }
    };

    *req.uri_mut() = new_uri;

    let response = state.dav_handler.handle(req).await.map(Body::new);

    if is_upload && response.status().is_success() {
        api::events::notify_file_updated(&state, &username).await;
    }

    response
}

fn is_allowed_dav_path(path: &str) -> bool {
    path == "/dav/" || path == format!("/dav/{DB_FILE_NAME}")
}

// https://http.dev/webdav
fn is_allowed_method(method: &Method) -> bool {
    matches!(
        *method,
        Method::OPTIONS | Method::GET | Method::HEAD | Method::PUT
    ) || method.as_str() == "PROPFIND"
        || method.as_str() == "LOCK"
        || method.as_str() == "UNLOCK"
}

fn content_length(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(axum::http::header::CONTENT_LENGTH)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}

fn rewrite_dav_uri(uri: &Uri, username: &str) -> Result<Uri, String> {
    let path = uri.path();
    let suffix = path
        .strip_prefix("/dav")
        .ok_or_else(|| "invalid dav path".to_string())?;

    let rewritten_path = if suffix.is_empty() {
        format!("/{username}/")
    } else if suffix.starts_with('/') {
        format!("/{username}{suffix}")
    } else {
        format!("/{username}/{suffix}")
    };

    let new_uri = match uri.query() {
        Some(query) => format!("{rewritten_path}?{query}"),
        None => rewritten_path,
    };

    new_uri.parse::<Uri>().map_err(|e| e.to_string())
}

fn unauthorized() -> Response {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(WWW_AUTHENTICATE, r#"Basic realm="webdav""#)
        .body(Body::from("Unauthorized"))
        .unwrap()
}
