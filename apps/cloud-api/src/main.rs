use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use base64::Engine;
use futures_util::TryStreamExt;
use rand::{distr::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::{env, path::PathBuf};
use tokio::{fs, io::AsyncWriteExt};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

const MAX_PROJECT_BYTES: usize = 32 * 1024 * 1024;
const MAX_MEDIA_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Clone)]
struct AppState {
    db: PgPool,
    media_root: PathBuf,
    internal_secret: String,
}

#[derive(Debug)]
struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1}))).into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;

#[derive(Serialize)]
struct ProjectSummary {
    id: String,
    name: String,
    thumbnail: Option<String>,
    duration: Option<Value>,
    created_at: Option<Value>,
    updated_at: Option<Value>,
}

#[derive(Serialize, Deserialize, Clone)]
struct MediaMetadata {
    id: String,
    name: String,
    #[serde(rename = "type")]
    media_type: String,
    size: u64,
    #[serde(rename = "lastModified")]
    last_modified: i64,
    width: Option<u32>,
    height: Option<u32>,
    duration: Option<f64>,
    #[serde(rename = "hasAudio")]
    has_audio: Option<bool>,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
}

fn api_error(status: StatusCode, message: impl Into<String>) -> ApiError {
    ApiError(status, message.into())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn json_error(error: sqlx::Error) -> ApiError {
    eprintln!("cloud api database error: {error}");
    api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database operation failed")
}

async fn authenticate(headers: &HeaderMap, state: &AppState) -> ApiResult<String> {
    let internal = headers
        .get("x-opencut-internal-secret")
        .and_then(|value| value.to_str().ok());
    if internal == Some(state.internal_secret.as_str()) {
        if let Some(user_id) = headers
            .get("x-opencut-user-id")
            .and_then(|value| value.to_str().ok())
            .filter(|value| valid_id(value))
        {
            return Ok(user_id.to_owned());
        }
    }

    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| value.starts_with("oc_"))
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "Sign in to access cloud projects"))?;
    let token_hash = hex_digest(token.as_bytes());
    let row = sqlx::query("SELECT user_id FROM project_api_tokens WHERE token_hash = $1")
        .bind(token_hash)
        .fetch_optional(&state.db)
        .await
        .map_err(json_error)?
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "Invalid MCP access token"))?;
    let user_id: String = row.try_get("user_id").map_err(json_error)?;
    Ok(user_id)
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn project_summary(payload: &Value) -> ApiResult<ProjectSummary> {
    let metadata = payload
        .get("metadata")
        .and_then(Value::as_object)
        .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, "Project metadata is missing"))?;
    let id = metadata
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, "Project ID is missing"))?;
    if !valid_id(id) {
        return Err(api_error(StatusCode::BAD_REQUEST, "Project ID is invalid"));
    }
    let name = metadata
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("Untitled Project")
        .chars()
        .take(256)
        .collect::<String>();
    Ok(ProjectSummary {
        id: id.to_owned(),
        name,
        thumbnail: metadata.get("thumbnail").and_then(Value::as_str).map(str::to_owned),
        duration: metadata.get("duration").cloned(),
        created_at: metadata.get("createdAt").cloned(),
        updated_at: metadata.get("updatedAt").cloned(),
    })
}

async fn ensure_owned_project(state: &AppState, user_id: &str, project_id: &str) -> ApiResult<Value> {
    let row = sqlx::query("SELECT payload FROM cloud_projects WHERE id = $1 AND user_id = $2")
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(json_error)?
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Project not found"))?;
    row.try_get("payload").map_err(json_error)
}

async fn list_projects(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let user_id = authenticate(&headers, &state).await?;
    let rows = sqlx::query("SELECT payload FROM cloud_projects WHERE user_id = $1 ORDER BY updated_at DESC")
        .bind(user_id)
        .fetch_all(&state.db)
        .await
        .map_err(json_error)?;
    let mut projects = Vec::with_capacity(rows.len());
    for row in rows {
        let payload: Value = row.try_get("payload").map_err(json_error)?;
        if let Ok(summary) = project_summary(&payload) {
            projects.push(json!({
                "id": summary.id,
                "name": summary.name,
                "thumbnail": summary.thumbnail,
                "duration": summary.duration,
                "createdAt": summary.created_at,
                "updatedAt": summary.updated_at,
            }));
        }
    }
    Ok(Json(json!({"projects": projects})))
}

async fn get_project(
    State(state): State<AppState>, headers: HeaderMap, Path(project_id): Path<String>,
) -> ApiResult<Json<Value>> {
    let user_id = authenticate(&headers, &state).await?;
    let payload = ensure_owned_project(&state, &user_id, &project_id).await?;
    Ok(Json(json!({"project": payload})))
}

async fn save_project(
    State(state): State<AppState>, headers: HeaderMap, Path(project_id): Path<String>, body: Bytes,
) -> ApiResult<Json<Value>> {
    let user_id = authenticate(&headers, &state).await?;
    if !valid_id(&project_id) || body.len() > MAX_PROJECT_BYTES {
        return Err(api_error(StatusCode::BAD_REQUEST, "Project payload is invalid or too large"));
    }
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|_| api_error(StatusCode::BAD_REQUEST, "Project payload is not valid JSON"))?;
    let summary = project_summary(&payload)?;
    if summary.id != project_id {
        return Err(api_error(StatusCode::BAD_REQUEST, "Project ID does not match payload"));
    }
    let now = chrono_now();
    let result = sqlx::query(
        "INSERT INTO cloud_projects (id, user_id, name, payload, updated_at, created_at) VALUES ($1,$2,$3,$4,$5,$5) \
         ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, payload = EXCLUDED.payload, updated_at = EXCLUDED.updated_at \
         WHERE cloud_projects.user_id = EXCLUDED.user_id",
    )
    .bind(&project_id)
    .bind(&user_id)
    .bind(summary.name)
    .bind(payload)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(json_error)?;
    if result.rows_affected() == 0 {
        return Err(api_error(StatusCode::FORBIDDEN, "Project belongs to another account"));
    }
    Ok(Json(json!({"success": true, "id": project_id})))
}

async fn delete_project(
    State(state): State<AppState>, headers: HeaderMap, Path(project_id): Path<String>,
) -> ApiResult<StatusCode> {
    let user_id = authenticate(&headers, &state).await?;
    let result = sqlx::query("DELETE FROM cloud_projects WHERE id = $1 AND user_id = $2")
        .bind(&project_id)
        .bind(&user_id)
        .execute(&state.db)
        .await
        .map_err(json_error)?;
    if result.rows_affected() == 0 {
        return Err(api_error(StatusCode::NOT_FOUND, "Project not found"));
    }
    let directory = state.media_root.join(&project_id);
    let _ = fs::remove_dir_all(directory).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn upload_media(
    State(state): State<AppState>, headers: HeaderMap, Path((project_id, media_id)): Path<(String, String)>, body: Body,
) -> ApiResult<Json<Value>> {
    let user_id = authenticate(&headers, &state).await?;
    ensure_owned_project(&state, &user_id, &project_id).await?;
    if !valid_id(&project_id) || !valid_id(&media_id) {
        return Err(api_error(StatusCode::BAD_REQUEST, "Invalid media ID"));
    }
    let encoded = headers
        .get("x-opencut-media-meta")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, "Media metadata is required"))?;
    let metadata_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| api_error(StatusCode::BAD_REQUEST, "Media metadata is invalid"))?;
    let metadata: MediaMetadata = serde_json::from_slice(&metadata_bytes)
        .map_err(|_| api_error(StatusCode::BAD_REQUEST, "Media metadata is invalid"))?;
    if metadata.id != media_id || metadata.size > MAX_MEDIA_BYTES {
        return Err(api_error(StatusCode::BAD_REQUEST, "Media metadata does not match upload"));
    }

    let project_dir = state.media_root.join(&project_id);
    fs::create_dir_all(&project_dir).await.map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Could not prepare media storage"))?;
    let temp_path = project_dir.join(format!(".{media_id}.{}.tmp", Uuid::new_v4()));
    let final_path = project_dir.join(&media_id);
    let mut file = fs::File::create(&temp_path).await.map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Could not write media"))?;
    let mut stream = body.into_data_stream();
    let mut written = 0_u64;
    loop {
        let chunk: Option<Bytes> = stream.try_next().await.map_err(|_| api_error(StatusCode::BAD_REQUEST, "Media upload stream failed"))?;
        let Some(chunk) = chunk else { break };
        written = written.saturating_add(chunk.len() as u64);
        if written > MAX_MEDIA_BYTES {
            let _ = fs::remove_file(&temp_path).await;
            return Err(api_error(StatusCode::PAYLOAD_TOO_LARGE, "Media exceeds the 2 GiB upload limit"));
        }
        file.write_all(&chunk).await.map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Could not write media"))?;
    }
    file.flush().await.map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Could not finish media upload"))?;
    drop(file);
    if written != metadata.size {
        let _ = fs::remove_file(&temp_path).await;
        return Err(api_error(StatusCode::BAD_REQUEST, "Uploaded file size does not match metadata"));
    }
    fs::rename(&temp_path, &final_path).await.map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Could not commit media upload"))?;
    let metadata_json = serde_json::to_value(&metadata).map_err(|_| api_error(StatusCode::BAD_REQUEST, "Media metadata is invalid"))?;
    sqlx::query("INSERT INTO cloud_media (project_id, id, metadata) VALUES ($1,$2,$3) ON CONFLICT (project_id,id) DO UPDATE SET metadata = EXCLUDED.metadata")
        .bind(&project_id).bind(&media_id).bind(metadata_json).execute(&state.db).await.map_err(json_error)?;
    Ok(Json(json!({"success": true, "id": media_id, "size": written})))
}

async fn list_media(
    State(state): State<AppState>, headers: HeaderMap, Path(project_id): Path<String>,
) -> ApiResult<Json<Value>> {
    let user_id = authenticate(&headers, &state).await?;
    ensure_owned_project(&state, &user_id, &project_id).await?;
    list_media_inner(&state, &project_id).await
}

async fn list_media_inner(state: &AppState, project_id: &str) -> ApiResult<Json<Value>> {
    let rows = sqlx::query("SELECT metadata FROM cloud_media WHERE project_id = $1 ORDER BY id")
        .bind(project_id).fetch_all(&state.db).await.map_err(json_error)?;
    let media = rows.into_iter().filter_map(|row| row.try_get::<Value, _>("metadata").ok()).collect::<Vec<_>>();
    Ok(Json(json!({"assets": media})))
}

async fn download_media(
    State(state): State<AppState>, headers: HeaderMap, Path((project_id, media_id)): Path<(String, String)>,
) -> ApiResult<Response> {
    let user_id = authenticate(&headers, &state).await?;
    ensure_owned_project(&state, &user_id, &project_id).await?;
    download_media_inner(&state, &project_id, &media_id).await
}

async fn download_media_inner(state: &AppState, project_id: &str, media_id: &str) -> ApiResult<Response> {
    if !valid_id(project_id) || !valid_id(media_id) {
        return Err(api_error(StatusCode::BAD_REQUEST, "Invalid media ID"));
    }
    let row = sqlx::query("SELECT metadata FROM cloud_media WHERE project_id = $1 AND id = $2")
        .bind(project_id).bind(media_id).fetch_optional(&state.db).await.map_err(json_error)?
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Media not found"))?;
    let metadata: Value = row.try_get("metadata").map_err(json_error)?;
    let content_type = metadata.get("mimeType").and_then(Value::as_str)
        .or_else(|| metadata.get("mime_type").and_then(Value::as_str))
        .unwrap_or("application/octet-stream");
    let file_path = state.media_root.join(project_id).join(media_id);
    let file = fs::File::open(&file_path).await
        .map_err(|_| api_error(StatusCode::NOT_FOUND, "Media file not found"))?;
    let file_size = fs::metadata(&file_path).await.map_err(|_| api_error(StatusCode::NOT_FOUND, "Media file not found"))?.len();
    let mut response = Response::new(Body::from_stream(ReaderStream::new(file)));
    response.headers_mut().insert(header::CONTENT_TYPE, content_type.parse().unwrap_or_else(|_| "application/octet-stream".parse().unwrap()));
    response.headers_mut().insert(header::CACHE_CONTROL, "private, no-store".parse().unwrap());
    if let Ok(value) = file_size.to_string().parse() {
        response.headers_mut().insert(header::CONTENT_LENGTH, value);
    }
    Ok(response)
}

async fn create_share(
    State(state): State<AppState>, headers: HeaderMap, Path(project_id): Path<String>,
) -> ApiResult<Json<Value>> {
    let user_id = authenticate(&headers, &state).await?;
    ensure_owned_project(&state, &user_id, &project_id).await?;
    let token = Uuid::new_v4().simple().to_string();
    sqlx::query("INSERT INTO project_shares (project_id, token, user_id) VALUES ($1,$2,$3) ON CONFLICT (project_id) DO UPDATE SET token = EXCLUDED.token, user_id = EXCLUDED.user_id")
        .bind(&project_id).bind(&token).bind(user_id).execute(&state.db).await.map_err(json_error)?;
    Ok(Json(json!({"token": token})))
}

async fn revoke_share(
    State(state): State<AppState>, headers: HeaderMap, Path(project_id): Path<String>,
) -> ApiResult<StatusCode> {
    let user_id = authenticate(&headers, &state).await?;
    let result = sqlx::query("DELETE FROM project_shares WHERE project_id = $1 AND user_id = $2")
        .bind(project_id).bind(user_id).execute(&state.db).await.map_err(json_error)?;
    if result.rows_affected() == 0 { return Err(api_error(StatusCode::NOT_FOUND, "Share link not found")); }
    Ok(StatusCode::NO_CONTENT)
}

async fn get_shared_project(
    State(state): State<AppState>, Path(token): Path<String>,
) -> ApiResult<Json<Value>> {
    let row = sqlx::query("SELECT p.payload FROM project_shares s JOIN cloud_projects p ON p.id = s.project_id WHERE s.token = $1")
        .bind(token).fetch_optional(&state.db).await.map_err(json_error)?
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Share link not found or revoked"))?;
    let project: Value = row.try_get("payload").map_err(json_error)?;
    let project_id = project_summary(&project)?.id;
    let media = list_media_inner(&state, &project_id).await?.0;
    Ok(Json(json!({"project": project, "assets": media.get("assets").cloned().unwrap_or(json!([]))})))
}

async fn get_shared_media(
    State(state): State<AppState>, Path((token, media_id)): Path<(String, String)>,
) -> ApiResult<Response> {
    let row = sqlx::query("SELECT project_id FROM project_shares WHERE token = $1")
        .bind(token).fetch_optional(&state.db).await.map_err(json_error)?
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Share link not found or revoked"))?;
    let project_id: String = row.try_get("project_id").map_err(json_error)?;
    download_media_inner(&state, &project_id, &media_id).await
}

async fn create_api_token(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let user_id = authenticate(&headers, &state).await?;
    // MCP credentials are shown once; only a hash is retained on the server.
    let suffix: String = rand::rng().sample_iter(&Alphanumeric).take(40).map(char::from).collect();
    let token = format!("oc_{suffix}");
    let token_hash = hex_digest(token.as_bytes());
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO project_api_tokens (id, user_id, token_hash, created_at) VALUES ($1,$2,$3,$4)")
        .bind(&id).bind(user_id).bind(token_hash).bind(chrono_now()).execute(&state.db).await.map_err(json_error)?;
    Ok(Json(json!({"id": id, "token": token})))
}

async fn list_api_tokens(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let user_id = authenticate(&headers, &state).await?;
    let rows = sqlx::query("SELECT id, created_at FROM project_api_tokens WHERE user_id = $1 ORDER BY created_at DESC")
        .bind(user_id).fetch_all(&state.db).await.map_err(json_error)?;
    let tokens = rows.into_iter().map(|row| json!({"id": row.try_get::<String,_>("id").unwrap_or_default(), "createdAt": row.try_get::<chrono::DateTime<chrono::Utc>,_>("created_at").ok()})).collect::<Vec<_>>();
    Ok(Json(json!({"tokens": tokens})))
}

async fn delete_api_token(State(state): State<AppState>, headers: HeaderMap, Path(token_id): Path<String>) -> ApiResult<StatusCode> {
    let user_id = authenticate(&headers, &state).await?;
    let result = sqlx::query("DELETE FROM project_api_tokens WHERE id = $1 AND user_id = $2")
        .bind(token_id).bind(user_id).execute(&state.db).await.map_err(json_error)?;
    if result.rows_affected() == 0 { return Err(api_error(StatusCode::NOT_FOUND, "API token not found")); }
    Ok(StatusCode::NO_CONTENT)
}

fn chrono_now() -> chrono::DateTime<chrono::Utc> { chrono::Utc::now() }

async fn health() -> &'static str { "OK" }

#[tokio::main]
async fn main() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let internal_secret = env::var("PROJECTS_INTERNAL_SECRET").expect("PROJECTS_INTERNAL_SECRET is required");
    let media_root = PathBuf::from(env::var("MEDIA_ROOT").unwrap_or_else(|_| "/data/media".to_owned()));
    let db = PgPoolOptions::new().max_connections(10).connect(&database_url).await.expect("connect to PostgreSQL");
    sqlx::query("CREATE TABLE IF NOT EXISTS users (id text PRIMARY KEY, name text NOT NULL, email text NOT NULL UNIQUE, email_verified boolean NOT NULL DEFAULT false, image text, created_at timestamp NOT NULL DEFAULT now(), updated_at timestamp NOT NULL DEFAULT now())").execute(&db).await.expect("create auth users table");
    sqlx::query("CREATE TABLE IF NOT EXISTS sessions (id text PRIMARY KEY, expires_at timestamp NOT NULL, token text NOT NULL UNIQUE, created_at timestamp NOT NULL, updated_at timestamp NOT NULL, ip_address text, user_agent text, user_id text NOT NULL REFERENCES users(id) ON DELETE CASCADE)").execute(&db).await.expect("create auth sessions table");
    sqlx::query("CREATE TABLE IF NOT EXISTS accounts (id text PRIMARY KEY, account_id text NOT NULL, provider_id text NOT NULL, user_id text NOT NULL REFERENCES users(id) ON DELETE CASCADE, access_token text, refresh_token text, id_token text, access_token_expires_at timestamp, refresh_token_expires_at timestamp, scope text, password text, created_at timestamp NOT NULL, updated_at timestamp NOT NULL)").execute(&db).await.expect("create auth accounts table");
    sqlx::query("CREATE TABLE IF NOT EXISTS verifications (id text PRIMARY KEY, identifier text NOT NULL, value text NOT NULL, expires_at timestamp NOT NULL, created_at timestamp DEFAULT now(), updated_at timestamp DEFAULT now())").execute(&db).await.expect("create auth verifications table");
    sqlx::query("CREATE TABLE IF NOT EXISTS feedback (id text PRIMARY KEY, message text NOT NULL, created_at timestamp NOT NULL DEFAULT now())").execute(&db).await.expect("create feedback table");
    sqlx::query("CREATE TABLE IF NOT EXISTS waitlist (id text PRIMARY KEY, email text NOT NULL UNIQUE, created_at timestamp NOT NULL DEFAULT now())").execute(&db).await.expect("create waitlist table");
    sqlx::query("CREATE TABLE IF NOT EXISTS cloud_projects (id text PRIMARY KEY, user_id text NOT NULL, name text NOT NULL, payload jsonb NOT NULL, created_at timestamptz NOT NULL, updated_at timestamptz NOT NULL)").execute(&db).await.expect("create cloud_projects table");
    sqlx::query("CREATE INDEX IF NOT EXISTS cloud_projects_user_updated ON cloud_projects(user_id, updated_at DESC)").execute(&db).await.expect("index cloud_projects");
    sqlx::query("CREATE TABLE IF NOT EXISTS cloud_media (project_id text NOT NULL REFERENCES cloud_projects(id) ON DELETE CASCADE, id text NOT NULL, metadata jsonb NOT NULL, PRIMARY KEY(project_id, id))").execute(&db).await.expect("create cloud_media table");
    sqlx::query("CREATE TABLE IF NOT EXISTS project_shares (project_id text PRIMARY KEY REFERENCES cloud_projects(id) ON DELETE CASCADE, token text NOT NULL UNIQUE, user_id text NOT NULL, created_at timestamptz NOT NULL DEFAULT now())").execute(&db).await.expect("create project_shares table");
    sqlx::query("CREATE TABLE IF NOT EXISTS project_api_tokens (id text PRIMARY KEY, user_id text NOT NULL, token_hash text NOT NULL UNIQUE, created_at timestamptz NOT NULL)").execute(&db).await.expect("create project_api_tokens table");
    fs::create_dir_all(&media_root).await.expect("prepare persistent media storage");

    let state = AppState { db, media_root, internal_secret };
    let app = Router::new()
        .route("/health", get(health))
        .route("/projects", get(list_projects))
        .route("/projects/{project_id}", get(get_project).put(save_project).delete(delete_project).layer(axum::extract::DefaultBodyLimit::max(MAX_PROJECT_BYTES)))
        .route("/projects/{project_id}/media", get(list_media))
        .route("/projects/{project_id}/media/{media_id}", put(upload_media).get(download_media).delete(delete_media))
        .route("/projects/{project_id}/share", post(create_share).delete(revoke_share))
        .route("/share/{token}", get(get_shared_project))
        .route("/share/{token}/media/{media_id}", get(get_shared_media))
        .route("/tokens", get(list_api_tokens).post(create_api_token))
        .route("/tokens/{token_id}", delete(delete_api_token))
        .with_state(state);
    let address = format!("0.0.0.0:{}", env::var("PORT").unwrap_or_else(|_| "8080".to_owned()));
    let listener = tokio::net::TcpListener::bind(&address).await.expect("bind cloud api");
    println!("OpenCut cloud API listening on {address}");
    axum::serve(listener, app).await.expect("serve cloud API");
}

async fn delete_media(
    State(state): State<AppState>, headers: HeaderMap, Path((project_id, media_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let user_id = authenticate(&headers, &state).await?;
    ensure_owned_project(&state, &user_id, &project_id).await?;
    sqlx::query("DELETE FROM cloud_media WHERE project_id = $1 AND id = $2")
        .bind(&project_id).bind(&media_id).execute(&state.db).await.map_err(json_error)?;
    let _ = fs::remove_file(state.media_root.join(project_id).join(media_id)).await;
    Ok(StatusCode::NO_CONTENT)
}
