#![allow(dead_code)]
use axum::{extract::State, response::Json, routing::{get, post}, Router};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

// ── State ────────────────────────────────────────────────────
struct AppState {
    start_time: Instant,
    stats: Mutex<Stats>,
}

struct Stats {
    total_uploads: u64,
    total_downloads: u64,
    total_lists: u64,
    total_bucket_ops: u64,
    bytes_stored: u64,
    bytes_transferred: u64,
}

// ── Types ────────────────────────────────────────────────────
#[derive(Serialize)]
struct Health { status: String, version: String, uptime_secs: u64, total_ops: u64 }

// Upload
#[derive(Deserialize)]
#[allow(dead_code)]
struct UploadRequest {
    bucket: Option<String>,
    key: Option<String>,
    content_type: Option<String>,
    size_bytes: Option<u64>,
    metadata: Option<serde_json::Value>,
}
#[derive(Serialize)]
struct UploadResponse {
    job_id: String,
    status: String,
    bucket: String,
    key: String,
    object_id: String,
    etag: String,
    size_bytes: u64,
    content_type: String,
    url: String,
    elapsed_us: u128,
}

// Download
#[derive(Deserialize)]
#[allow(dead_code)]
struct DownloadRequest {
    bucket: Option<String>,
    key: Option<String>,
    version_id: Option<String>,
    presign_ttl_secs: Option<u64>,
}
#[derive(Serialize)]
struct DownloadResponse {
    job_id: String,
    status: String,
    bucket: String,
    key: String,
    object_id: String,
    size_bytes: u64,
    content_type: String,
    presigned_url: String,
    url_expires_secs: u64,
    elapsed_us: u128,
}

// List
#[derive(Deserialize)]
#[allow(dead_code)]
struct ListRequest {
    bucket: Option<String>,
    prefix: Option<String>,
    max_keys: Option<u32>,
    continuation_token: Option<String>,
}
#[derive(Serialize)]
struct ObjectEntry {
    key: String,
    size_bytes: u64,
    etag: String,
    content_type: String,
    last_modified: String,
}
#[derive(Serialize)]
struct ListResponse {
    bucket: String,
    prefix: String,
    key_count: u32,
    objects: Vec<ObjectEntry>,
    is_truncated: bool,
    next_continuation_token: Option<String>,
    elapsed_us: u128,
}

// Buckets
#[derive(Serialize)]
struct BucketInfo {
    name: String,
    region: String,
    object_count: u64,
    size_bytes: u64,
    versioning: bool,
    created_at: String,
}
#[derive(Serialize)]
struct BucketsResponse {
    buckets: Vec<BucketInfo>,
    total_buckets: u32,
    elapsed_us: u128,
}

// Stats
#[derive(Serialize)]
struct StatsResponse {
    total_uploads: u64,
    total_downloads: u64,
    total_lists: u64,
    total_bucket_ops: u64,
    bytes_stored: u64,
    bytes_transferred: u64,
}

// ── Main ─────────────────────────────────────────────────────
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "storage_engine=info".into()))
        .init();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        stats: Mutex::new(Stats {
            total_uploads: 0,
            total_downloads: 0,
            total_lists: 0,
            total_bucket_ops: 0,
            bytes_stored: 0,
            bytes_transferred: 0,
        }),
    });
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/storage/upload", post(upload))
        .route("/api/v1/storage/download", post(download))
        .route("/api/v1/storage/list", post(list))
        .route("/api/v1/storage/buckets", get(buckets))
        .route("/api/v1/storage/stats", get(stats))
        .layer(cors).layer(TraceLayer::new_for_http()).with_state(state);
    let addr = std::env::var("STORAGE_ADDR").unwrap_or_else(|_| "0.0.0.0:8134".into());
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Storage Engine on {addr}");
    axum::serve(listener, app).await.unwrap();
}

// ── Handlers ─────────────────────────────────────────────────
async fn health(State(s): State<Arc<AppState>>) -> Json<Health> {
    let st = s.stats.lock().unwrap();
    Json(Health {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_secs: s.start_time.elapsed().as_secs(),
        total_ops: st.total_uploads + st.total_downloads + st.total_lists + st.total_bucket_ops,
    })
}

async fn upload(State(s): State<Arc<AppState>>, Json(req): Json<UploadRequest>) -> Json<UploadResponse> {
    let t = Instant::now();
    let bucket = req.bucket.unwrap_or_else(|| "default".into());
    let key = req.key.unwrap_or_else(|| format!("objects/{}", uuid::Uuid::new_v4()));
    let content_type = req.content_type.unwrap_or_else(|| "application/octet-stream".into());
    let size = req.size_bytes.unwrap_or(0);
    let object_id = uuid::Uuid::new_v4().to_string();
    let etag = format!("{:x}", md5_stub(size));
    let url = format!("https://storage.alice-storage.io/{bucket}/{key}");

    {
        let mut st = s.stats.lock().unwrap();
        st.total_uploads += 1;
        st.bytes_stored += size;
    }

    Json(UploadResponse {
        job_id: uuid::Uuid::new_v4().to_string(),
        status: "stored".into(),
        bucket,
        key,
        object_id,
        etag,
        size_bytes: size,
        content_type,
        url,
        elapsed_us: t.elapsed().as_micros(),
    })
}

async fn download(State(s): State<Arc<AppState>>, Json(req): Json<DownloadRequest>) -> Json<DownloadResponse> {
    let t = Instant::now();
    let bucket = req.bucket.unwrap_or_else(|| "default".into());
    let key = req.key.unwrap_or_else(|| "objects/unknown".into());
    let ttl = req.presign_ttl_secs.unwrap_or(3600);
    let object_id = uuid::Uuid::new_v4().to_string();
    let size: u64 = 1_048_576; // 1 MiB placeholder
    let presigned = format!(
        "https://storage.alice-storage.io/{bucket}/{key}?X-Object-Id={object_id}&Expires={ttl}"
    );

    {
        let mut st = s.stats.lock().unwrap();
        st.total_downloads += 1;
        st.bytes_transferred += size;
    }

    Json(DownloadResponse {
        job_id: uuid::Uuid::new_v4().to_string(),
        status: "ready".into(),
        bucket,
        key,
        object_id,
        size_bytes: size,
        content_type: "application/octet-stream".into(),
        presigned_url: presigned,
        url_expires_secs: ttl,
        elapsed_us: t.elapsed().as_micros(),
    })
}

async fn list(State(s): State<Arc<AppState>>, Json(req): Json<ListRequest>) -> Json<ListResponse> {
    let t = Instant::now();
    let bucket = req.bucket.unwrap_or_else(|| "default".into());
    let prefix = req.prefix.unwrap_or_default();
    let max_keys = req.max_keys.unwrap_or(1000).min(1000);

    // Return representative sample objects
    let objects: Vec<ObjectEntry> = (0..3u32).map(|i| ObjectEntry {
        key: format!("{prefix}object-{i:04}.bin"),
        size_bytes: (i as u64 + 1) * 4096,
        etag: format!("{:x}", md5_stub(i as u64)),
        content_type: "application/octet-stream".into(),
        last_modified: "2026-03-09T00:00:00Z".into(),
    }).collect();
    let key_count = objects.len() as u32;

    s.stats.lock().unwrap().total_lists += 1;

    Json(ListResponse {
        bucket,
        prefix,
        key_count,
        objects,
        is_truncated: key_count >= max_keys,
        next_continuation_token: None,
        elapsed_us: t.elapsed().as_micros(),
    })
}

async fn buckets(State(s): State<Arc<AppState>>) -> Json<BucketsResponse> {
    let t = Instant::now();
    s.stats.lock().unwrap().total_bucket_ops += 1;

    let bucket_list = vec![
        BucketInfo {
            name: "default".into(),
            region: "ap-northeast-1".into(),
            object_count: 0,
            size_bytes: 0,
            versioning: false,
            created_at: "2026-03-09T00:00:00Z".into(),
        },
        BucketInfo {
            name: "archive".into(),
            region: "ap-northeast-1".into(),
            object_count: 0,
            size_bytes: 0,
            versioning: true,
            created_at: "2026-03-09T00:00:00Z".into(),
        },
    ];
    let total = bucket_list.len() as u32;

    Json(BucketsResponse {
        buckets: bucket_list,
        total_buckets: total,
        elapsed_us: t.elapsed().as_micros(),
    })
}

async fn stats(State(s): State<Arc<AppState>>) -> Json<StatsResponse> {
    let st = s.stats.lock().unwrap();
    Json(StatsResponse {
        total_uploads: st.total_uploads,
        total_downloads: st.total_downloads,
        total_lists: st.total_lists,
        total_bucket_ops: st.total_bucket_ops,
        bytes_stored: st.bytes_stored,
        bytes_transferred: st.bytes_transferred,
    })
}

// ── Helpers ──────────────────────────────────────────────────
/// Deterministic stub to generate an etag-like hex value.
fn md5_stub(seed: u64) -> u64 {
    seed.wrapping_mul(0x517cc1b727220a95).wrapping_add(0xdeadbeef)
}
