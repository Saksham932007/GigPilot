# GigPilot

A Local-First, Offline-First OS for Freelancers built with Rust, React Native, and CRDTs.

## Architecture

- **Backend**: Rust (Axum) for high-performance sync and WebSocket handling
- **Frontend**: React Native with WatermelonDB (local-first DB)
- **Database**: PostgreSQL (remote) + SQLite (local)
- **Sync**: Custom CRDT-based synchronization protocol
- **Storage**: MinIO (S3-compatible) for invoices and documents

## Prerequisites

- Rust 1.70+ (with cargo)
- Docker and Docker Compose
- PostgreSQL client tools (for migrations)

## Setup

### 1. Start Infrastructure Services

```bash
docker-compose up -d
```

This starts:
- PostgreSQL on port 5432
- Redis on port 6379
- MinIO on ports 9000 (API) and 9001 (Console)

### 2. Configure Environment

Copy `.env.example` to `.env` and update values:

```bash
cp .env.example .env
```

### 3. Run Database Migrations

```bash
cd gigpilot-core
sqlx migrate run
```

### 4. Build and Run

```bash
cargo build --release
cargo run
```

The server will start on `http://localhost:3000` (or as configured in `.env`).

## Project Structure

```
GigPilot/
├── gigpilot-core/          # Rust backend
│   ├── src/
│   │   ├── main.rs        # Application entry point
│   │   ├── auth.rs        # JWT authentication middleware
│   │   ├── db.rs          # Database connection pool
│   │   └── models/        # Database models
│   └── migrations/        # SQL migrations
├── docker-compose.yml     # Infrastructure services
└── Cargo.toml            # Workspace configuration
```

## Development

### Running Migrations

```bash
cd gigpilot-core
sqlx migrate add <migration_name>
sqlx migrate run
```

### Testing Database Connection

```bash
curl http://localhost:3000/health/db
```

## License

MIT OR Apache-2.0

