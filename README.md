# GigPilot

**A Local-First, Offline-First OS for Freelancers**

GigPilot is a distributed system built with Rust, React Native, and CRDTs that enables freelancers to manage their business completely offline, with seamless synchronization when connectivity is restored.

## 🏗️ Architecture Overview

GigPilot implements a **true offline-first architecture** where the local database (SQLite via WatermelonDB) is the source of truth, and the remote PostgreSQL database serves as a synchronization hub. This design ensures:

- **Zero-downtime operation**: Work continues even when offline
- **Conflict-free synchronization**: CRDT-based state management
- **Event-sourced audit trail**: Complete history of all changes
- **Multi-device support**: Sync across phones, tablets, and desktops

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                    React Native Frontend                     │
│  ┌──────────────────────────────────────────────────────┐   │
│  │         WatermelonDB (Local SQLite)                  │   │
│  │  • Users • Invoices • Sync Changes                   │   │
│  └──────────────────┬───────────────────────────────────┘   │
│                     │ Sync Protocol                         │
│                     ▼                                       │
│  ┌──────────────────────────────────────────────────────┐   │
│  │         Custom Sync Adapter                         │   │
│  │  • Pull Changes • Push Changes                      │   │
│  └──────────────────┬───────────────────────────────────┘   │
└─────────────────────┼───────────────────────────────────────┘
                      │ HTTPS / WebSocket
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                    Rust Backend (Axum)                       │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Sync Engine (Neural Sync)                           │   │
│  │  • Pull Endpoint • Push Endpoint                     │   │
│  │  • Conflict Resolution • Version Vectors             │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Chasing Agent (Background Worker)                   │   │
│  │  • State Machine • Email Generation                  │   │
│  │  • LLM Integration • Auto-chasing                    │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Contextual Estimator (RAG)                           │   │
│  │  • Vector Search • Embeddings                        │   │
│  │  • Similarity Search • Project Matching              │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                    PostgreSQL Database                       │
│  • Users • Invoices • Sync Changes                          │
│  • Embeddings (pgvector) • Row Level Security              │
└─────────────────────────────────────────────────────────────┘
```

## 🔄 Offline-First Sync Protocol

### How It Works

1. **Local-First Operations**
   - All reads and writes happen against the local SQLite database
   - No network calls required for normal operations
   - Instant UI updates, zero latency

2. **Background Synchronization**
   - Sync runs in the background when connectivity is available
   - Pull: Fetches changes from server since last sync
   - Push: Sends local changes to server
   - Automatic conflict resolution using version vectors

3. **Conflict Resolution**
   - **Version Vectors**: Each change includes a vector clock
   - **Last-Write-Wins**: By default, newer changes win
   - **Server-Wins**: Configurable strategy for critical data
   - **Client-Wins**: For user-initiated changes

4. **State Persistence**
   - All state stored in database (survives restarts)
   - Sync metadata tracks what's been synchronized
   - Changeset log for audit and replay

### Sync Flow Example

```
User creates invoice offline:
  Local DB: INSERT invoice (status: draft)
  Sync Changes: Record change with device_id + timestamp

User goes online:
  Push: Send change to server
  Server: Apply change, record in sync_changes
  Pull: Fetch any other changes from server
  Local DB: Merge changes, update UI
```

## 🧠 Contextual Estimator (RAG)

The Contextual Estimator uses **Retrieval-Augmented Generation (RAG)** to help freelancers estimate project costs based on similar past work.

### How It Works

1. **Embedding Storage**
   - When invoices/projects are created, text is embedded using OpenAI
   - Embeddings stored in PostgreSQL with pgvector extension
   - 1536-dimensional vectors for semantic search

2. **Similarity Search**
   - User queries: "Build a React Native app with auth"
   - System generates embedding for query
   - Searches for similar past projects using cosine similarity
   - Returns top matches with similarity scores

3. **Cost Estimation**
   - Analyzes similar past projects
   - Suggests pricing based on historical data
   - Considers complexity, duration, and client type

## 🤖 Chasing Agent

An intelligent background worker that automatically chases overdue invoices through a state machine:

```
Pending → Overdue → ChasingLevel1 (Polite) → ChasingLevel2 (Firm) → Paid
```

- **State Machine**: Automatic progression through chase levels
- **LLM Integration**: Generates personalized email content
- **Email Sending**: Integrates with email providers
- **Survives Restarts**: State persisted in database

## 🛠️ Technology Stack

### Backend
- **Rust** with **Axum** - High-performance async web framework
- **sqlx** - Type-safe SQL with compile-time query checking
- **PostgreSQL** - Primary database with pgvector extension
- **Redis** - Task queues and caching
- **MinIO** - S3-compatible object storage

### Frontend
- **React Native** - Cross-platform mobile framework
- **WatermelonDB** - Local-first database (SQLite)
- **TypeScript** - Type-safe frontend code

### AI/ML
- **OpenAI Embeddings** - Text embedding generation
- **pgvector** - Vector similarity search in PostgreSQL
- **Custom RAG** - Retrieval-augmented generation pipeline

## 📦 Prerequisites

- Rust 1.70+ (with cargo)
- Node.js 18+ (for frontend)
- Docker and Docker Compose
- PostgreSQL 14+ with pgvector extension

## 🚀 Quick Start

### 1. Start Infrastructure Services

```bash
docker-compose up -d
```

This starts:
- PostgreSQL on port 5432 (with pgvector)
- Redis on port 6379
- MinIO on ports 9000 (API) and 9001 (Console)

### 2. Configure Environment

```bash
cp .env.example .env
```

Update `.env` with your configuration:
- `DATABASE_URL` - PostgreSQL connection string
- `JWT_SECRET` - Secret for JWT tokens
- `OPENAI_API_KEY` - For embeddings (optional, uses mock if not set)

### 3. Run Database Migrations

```bash
cd gigpilot-core
sqlx migrate run
```

This creates all tables including:
- `users` - User accounts with RLS
- `invoices` - Invoice data with sync metadata
- `sync_changes` - Changeset log for sync
- `embeddings` - Vector embeddings for RAG

### 4. Build and Run Backend

```bash
cargo build --release
cargo run
```

The server starts on `http://localhost:3000`

### 5. Run Background Worker

In a separate terminal:

```bash
cargo run --bin worker
```

The worker processes overdue invoices and sends chase emails.

### 6. Setup Frontend

```bash
cd frontend
npm install
npm start
```

## 📊 Observability

GigPilot includes comprehensive logging and latency tracking:

- **LLM Latency**: Tracks time for embedding generation
- **DB Latency**: Tracks database query performance
- **Sync Metrics**: Monitors sync operation performance
- **Structured Logging**: JSON-formatted logs with timestamps

Example log output:
```
INFO gigpilot_core::rag::embeddings: LLM embedding generation took: 200ms
INFO gigpilot_core::rag::embeddings: Database insertion took: 15ms
INFO gigpilot_core::rag::embeddings: Total latency - LLM: 200ms, DB: 15ms
```

## 🔐 Security

- **Row Level Security (RLS)**: Database-level access control
- **JWT Authentication**: Secure token-based auth
- **Version Vectors**: Prevent sync conflicts and data corruption
- **Soft Deletes**: Preserve data for audit trail

## 📁 Project Structure

```
GigPilot/
├── gigpilot-core/              # Rust backend
│   ├── src/
│   │   ├── main.rs             # API server entry point
│   │   ├── bin/worker.rs       # Background worker entry point
│   │   ├── auth.rs             # JWT authentication
│   │   ├── db.rs                # Database connection pool
│   │   ├── sync/                # Sync engine
│   │   │   ├── pull.rs         # Pull endpoint
│   │   │   ├── push.rs         # Push endpoint
│   │   │   └── conflict.rs      # Conflict resolution
│   │   ├── worker/              # Chasing agent
│   │   │   ├── scheduler.rs    # Job scheduler
│   │   │   ├── state_machine.rs # Chase state machine
│   │   │   └── services.rs     # LLM/Email mocks
│   │   ├── rag/                 # Contextual estimator
│   │   │   ├── embeddings.rs   # Embedding storage
│   │   │   └── search.rs        # Similarity search
│   │   └── models/              # Database models
│   └── migrations/              # SQL migrations
├── frontend/                    # React Native app
│   ├── src/
│   │   ├── schema/             # WatermelonDB schema
│   │   ├── sync/               # Sync adapter
│   │   └── api/                # API client
└── docker-compose.yml           # Infrastructure services
```

## 🧪 Testing

```bash
# Run backend tests
cd gigpilot-core
cargo test

# Run sync tests (requires database)
DATABASE_URL=postgresql://... cargo test -- --ignored

# Run frontend tests
cd frontend
npm test
```

## 📝 API Endpoints

### Authentication
- `POST /auth/register` - Register new user
- `POST /auth/login` - Login and get JWT token

### Sync
- `GET /sync/pull?last_pulled_at=<timestamp>` - Pull changes
- `POST /sync/push` - Push local changes

### Health
- `GET /health` - Server health check
- `GET /health/db` - Database health check

## 🎯 Key Features

✅ **Offline-First**: Work without internet connection  
✅ **Multi-Device Sync**: Seamless sync across devices  
✅ **Conflict Resolution**: Automatic conflict handling  
✅ **AI-Powered**: Contextual estimation with RAG  
✅ **Auto-Chasing**: Intelligent invoice follow-up  
✅ **Event-Sourced**: Complete audit trail  
✅ **Type-Safe**: Compile-time query checking with sqlx  
✅ **Production-Ready**: Proper error handling, logging, RLS  

## 📄 License

MIT OR Apache-2.0

## 🤝 Contributing

This is a demonstration project showcasing:
- Offline-first architecture patterns
- CRDT-based synchronization
- Event sourcing
- RAG implementation
- Background job processing

For production use, additional considerations:
- Rate limiting
- API versioning
- Monitoring and alerting
- Backup and disaster recovery
- Load balancing
- Caching strategies

---

**Built with ❤️ for freelancers who value their time and data sovereignty.**
