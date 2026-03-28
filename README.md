# ALICE Storage SaaS

Cloud object storage platform powered by the ALICE ecosystem. Upload, download, list, and manage objects across buckets via a simple REST API.

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

## Status

| Check | Status |
|-------|--------|
| `cargo check` | passing |
| API health | `/health` |

## Quick Start

```bash
docker compose up -d
```

API Gateway: http://localhost:8080
Storage Engine: http://localhost:8134

## Architecture

```
Client
  |
  v
API Gateway     :8080
  |
  v
Storage Engine  :8134
(FileSystem + ObjectStore)
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/storage/upload` | Upload an object to a bucket |
| `POST` | `/api/v1/storage/download` | Generate presigned download URL |
| `POST` | `/api/v1/storage/list` | List objects in a bucket |
| `GET` | `/api/v1/storage/buckets` | List all buckets |
| `GET` | `/api/v1/storage/stats` | Storage statistics |
| `GET` | `/health` | Service health check |

### upload

```json
POST /api/v1/storage/upload
{
  "bucket": "default",
  "key": "images/photo.jpg",
  "content_type": "image/jpeg",
  "size_bytes": 204800
}
```

Response:
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "stored",
  "bucket": "default",
  "key": "images/photo.jpg",
  "object_id": "a3f7c2d1-...",
  "etag": "d41d8cd98f00b204",
  "size_bytes": 204800,
  "content_type": "image/jpeg",
  "url": "https://storage.alice-storage.io/default/images/photo.jpg",
  "elapsed_us": 38
}
```

### download

```json
POST /api/v1/storage/download
{
  "bucket": "default",
  "key": "images/photo.jpg",
  "presign_ttl_secs": 3600
}
```

### list

```json
POST /api/v1/storage/list
{
  "bucket": "default",
  "prefix": "images/",
  "max_keys": 100
}
```

### buckets

```
GET /api/v1/storage/buckets
```

Response:
```json
{
  "buckets": [
    {
      "name": "default",
      "region": "ap-northeast-1",
      "object_count": 0,
      "size_bytes": 0,
      "versioning": false,
      "created_at": "2026-03-09T00:00:00Z"
    }
  ],
  "total_buckets": 1
}
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `STORAGE_ADDR` | `0.0.0.0:8134` | Storage engine bind address |
| `CORE_ENGINE_URL` | `http://core-engine:8134` | Core engine URL for gateway |
| `JWT_SECRET` | `dev-secret-change-me` | JWT signing secret |

## License

AGPL-3.0. Commercial dual-license available — contact for pricing.
