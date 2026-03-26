-- =============================================================
-- user_presence: ephemeral per-session presence tracking
--
-- One row per active WebSocket connection (session), not per user.
-- A user is considered:
--   online  - has >= 1 fresh session where status = 'online'
--   idle    - has fresh sessions but ALL are status = 'idle'
--   offline - no fresh sessions (no rows, or all past the 60s threshold)
--
-- Heartbeat must be sent every ~20-30s from the client.
-- A background task deletes rows older than 60s to handle crashed clients.
-- =============================================================

CREATE TYPE presence_status AS ENUM ('online', 'idle');

CREATE TABLE IF NOT EXISTS user_presence (
    session_id        UUID            PRIMARY KEY DEFAULT uuidv7(),
    user_id           UUID            NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    status            presence_status NOT NULL DEFAULT 'online',
    last_heartbeat_at TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    connected_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_presence_user_id
    ON user_presence(user_id);

CREATE INDEX IF NOT EXISTS idx_user_presence_heartbeat
    ON user_presence(last_heartbeat_at);
