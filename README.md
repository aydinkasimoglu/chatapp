# ChatApp Backend

A real-time chat and social networking API server built with Rust, featuring WebSocket presence tracking, a friend/block system, and JWT authentication.

## Tech Stack

- **Runtime:** Tokio (async)
- **Framework:** Axum 0.8 (HTTP + WebSocket)
- **Database:** PostgreSQL via SQLx (compile-time verified queries)
- **Auth:** JWT (HS256) access tokens + rotating opaque refresh tokens
- **Password Hashing:** Argon2
- **Logging:** tracing + tracing-subscriber

## Getting Started

### Prerequisites

- Rust
- PostgreSQL
- [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) — `cargo install sqlx-cli --no-default-features -F postgres`

### Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `DATABASE_URL` | Yes | — | PostgreSQL connection string |
| `JWT_SECRET` | Yes | — | Secret key for JWT signing (min 32 bytes) |
| `HOST` | No | `0.0.0.0` | Bind address |
| `PORT` | No | `3000` | Listen port |
| `DB_MAX_CONNECTIONS` | No | `20` | Max database pool connections |
| `RUST_LOG` | No | `info` | Log level filter |

### Setup

```bash
# Clone and enter the project
cd chatapp

# Run database migrations
sqlx migrate run

# Build and run
cargo run
```

For production:

```bash
cargo build --release
./target/release/chatapp
```

## Architecture

```
src/
├── main.rs          # Server startup, routing, background tasks
├── state.rs         # Shared application state (DB pool, services)
├── error.rs         # ServiceError → HTTP response mapping
├── models.rs        # DB models, request/response DTOs, enums
├── extractors.rs    # AuthenticatedUser JWT extractor
├── routes/          # Route definitions (thin wiring layer)
├── handlers/        # Request handlers (parse input, call service, return response)
├── services/        # Business logic
└── repositories/    # Database queries (SQLx)
```

Follows a layered architecture: **Routes → Handlers → Services → Repositories**.

## API Endpoints

### Authentication

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| POST | `/signup` | — | Register a new user |
| POST | `/login` | — | Login, returns access + refresh tokens |
| POST | `/auth/refresh` | — | Exchange refresh token for new token pair |
| POST | `/auth/logout` | Yes | Logout current device (with body) or all devices |

### Users

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| GET | `/users/` | Yes | List users (paginated: `?limit=&offset=`) |
| GET | `/users/{user_id}` | Yes | Get user profile |
| PUT | `/users/{user_id}` | Yes | Update username/email |
| DELETE | `/users/{user_id}` | Yes | Deactivate account (soft delete) |
| PUT | `/users/{user_id}/password` | Yes | Change password |

### Servers

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| POST | `/servers/` | Yes | Create server (creator becomes owner) |
| GET | `/servers/{server_id}` | Optional | Get server details |
| PUT | `/servers/{server_id}` | Owner | Update server |
| DELETE | `/servers/{server_id}` | Owner | Delete server |
| GET | `/servers/public` | Optional | List public servers |
| GET | `/servers/mine` | Yes | List servers the user is a member of |

### Friends

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| GET | `/friends/` | Yes | List accepted friends |
| GET | `/friends/online` | Yes | List online/idle friends with presence |
| POST | `/friends/requests` | Yes | Send friend request (by username) |
| GET | `/friends/requests/incoming` | Yes | List incoming pending requests |
| GET | `/friends/requests/outgoing` | Yes | List outgoing pending requests |
| PUT | `/friends/requests/{id}/accept` | Yes | Accept a friend request |
| PUT | `/friends/requests/{id}/reject` | Yes | Reject a friend request |
| DELETE | `/friends/requests/{id}/cancel` | Yes | Cancel a sent request |
| DELETE | `/friends/{id}` | Yes | Remove a friend |

### Blocks

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| GET | `/blocks/` | Yes | List blocked users |
| POST | `/blocks/{target_user_id}` | Yes | Block a user (removes any friendship) |
| DELETE | `/blocks/{target_user_id}` | Yes | Unblock a user |

### WebSocket

| Endpoint | Auth | Description |
|---|---|---|
| `/ws/presence` | Yes | Presence tracking — heartbeats, friend online/offline notifications |
| `/ws/{room_name}` | Yes | Chat room — message broadcasting |

## WebSocket Protocol

### Presence (`/ws/presence`)

**Client → Server** (send every 20–30s):
```json
{ "type": "heartbeat", "status": "online" }
{ "type": "heartbeat", "status": "idle" }
```

**Server → Client:**
```json
{ "type": "online_friends", "friends": [{ "user_id": "...", "username": "...", "status": "online", "last_heartbeat_at": "..." }] }
{ "type": "presence_update", "user_id": "...", "username": "...", "status": "online" }
```

On connect, the server sends an initial snapshot of online friends. Subsequent `presence_update` messages are pushed when friends come online or go offline.

### Chat Room (`/ws/{room_name}`)

**Client → Server:**
```json
{ "type": "message", "content": "Hello" }
```

**Server → Client:**
```
username: Hello
```

Messages are broadcast to all connected users in the room.

## Database Schema

Six core tables managed via SQLx migrations:

- **users** — Accounts with soft-delete (`is_active`)
- **servers** — Chat servers with public/private visibility
- **server_members** — User–server membership with roles (`owner`, `admin`, `moderator`, `member`)
- **friendships** — Friend requests and relationships (`pending`, `accepted`, `rejected`)
- **user_blocks** — Directional blocking
- **user_presence** — Per-WebSocket-session heartbeat tracking
- **refresh_tokens** — SHA-256 hashed refresh tokens with expiry

Run migrations:

```bash
sqlx migrate run
```

## Authentication

- **Access tokens:** Short-lived (15 min) JWTs passed via `Authorization: Bearer <token>`
- **Refresh tokens:** Long-lived (30 day) opaque UUIDs, stored as SHA-256 hashes in the database
- **Token rotation:** Each refresh consumes the old token and issues a new pair
- **Session limit:** Max 10 concurrent refresh tokens per user; oldest pruned on new login
- **Passwords:** Hashed with Argon2, validated at 8–128 characters

## Background Tasks

- **Presence cleanup:** Every 15 seconds, removes stale sessions (no heartbeat for 30+ seconds)
- **Token cleanup:** Every 24 hours, deletes expired refresh tokens

## Error Responses

All errors follow a consistent format:

```json
{ "error": "Human-readable error message" }
```

| Status | Meaning |
|---|---|
| 400 | Validation error |
| 401 | Unauthorized / invalid token |
| 404 | Resource not found |
| 409 | Conflict (duplicate, already exists, blocked) |
| 500 | Internal server error |
