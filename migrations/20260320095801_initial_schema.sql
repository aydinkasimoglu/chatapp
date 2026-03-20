-- Enable pgcrypto if needed for any hashing
CREATE EXTENSION IF NOT EXISTS pgcrypto;


-- =============================================================
-- USERS
-- =============================================================
CREATE TABLE users (
    user_id     UUID        PRIMARY KEY DEFAULT uuidv7(),

    username    VARCHAR(50)  NOT NULL,
    email       VARCHAR(255) NOT NULL,

    -- Never store plain passwords — store hashes only
    password_hash VARCHAR(255) NOT NULL,

    is_active   BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_users_username UNIQUE (username),
    CONSTRAINT uq_users_email    UNIQUE (email),
    CONSTRAINT chk_users_email   CHECK  (email ~* '^[^@]+@[^@]+\.[^@]+$')
);

-- Partial index: fast lookups for active users only
CREATE INDEX idx_users_active ON users (username) WHERE is_active = TRUE;


-- =============================================================
-- SERVERS
-- =============================================================
CREATE TABLE servers (
    server_id   UUID        PRIMARY KEY DEFAULT uuidv7(),
    owner_id    UUID        NOT NULL,

    name        VARCHAR(100) NOT NULL,
    description TEXT,
    is_public   BOOLEAN     NOT NULL DEFAULT TRUE,

    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_servers_owner
        FOREIGN KEY (owner_id) REFERENCES users(user_id)
        ON DELETE RESTRICT,   -- Don't silently delete servers when owner is deleted

    CONSTRAINT uq_servers_owner_name UNIQUE (owner_id, name)
);

CREATE INDEX idx_servers_owner ON servers (owner_id);


-- =============================================================
-- SERVER MEMBERS (junction table)
-- =============================================================
CREATE TYPE member_role AS ENUM ('owner', 'admin', 'moderator', 'member');

CREATE TABLE server_members (
    user_id     UUID        NOT NULL,
    server_id   UUID        NOT NULL,

    nickname    VARCHAR(100),
    role        member_role NOT NULL DEFAULT 'member',

    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (user_id, server_id),

    CONSTRAINT fk_sm_user
        FOREIGN KEY (user_id)   REFERENCES users(user_id)     ON DELETE CASCADE,
    CONSTRAINT fk_sm_server
        FOREIGN KEY (server_id) REFERENCES servers(server_id) ON DELETE CASCADE
);

-- Useful for "list all members of a server" queries
CREATE INDEX idx_sm_server ON server_members (server_id);
-- Useful for "list all servers a user is in" queries  
CREATE INDEX idx_sm_user   ON server_members (user_id);


-- =============================================================
-- AUTO-UPDATE updated_at via trigger
-- =============================================================
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trg_servers_updated_at
    BEFORE UPDATE ON servers
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- =============================================================
-- CHANNELS (belong to a server)
-- =============================================================
CREATE TYPE channel_type AS ENUM ('text', 'announcement');

CREATE TABLE channels (
    channel_id  UUID         PRIMARY KEY DEFAULT uuidv7(),
    server_id   UUID         NOT NULL,
    name        VARCHAR(100) NOT NULL,
    topic       TEXT,
    type        channel_type NOT NULL DEFAULT 'text',
    position    INT          NOT NULL DEFAULT 0,  -- display order
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_channels_server
        FOREIGN KEY (server_id) REFERENCES servers(server_id)
        ON DELETE CASCADE,

    CONSTRAINT uq_channels_server_name UNIQUE (server_id, name)
);

CREATE INDEX idx_channels_server ON channels (server_id, position);


-- =============================================================
-- MESSAGES
-- =============================================================
CREATE TABLE messages (
    message_id  UUID        PRIMARY KEY DEFAULT uuidv7(),
    channel_id  UUID        NOT NULL,
    user_id     UUID        NOT NULL,
    content     TEXT        NOT NULL,
    edited_at   TIMESTAMPTZ,              -- NULL means never edited
    deleted_at  TIMESTAMPTZ,              -- NULL means not deleted (soft delete)
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_messages_channel
        FOREIGN KEY (channel_id) REFERENCES channels(channel_id)
        ON DELETE CASCADE,

    CONSTRAINT fk_messages_user
        FOREIGN KEY (user_id) REFERENCES users(user_id)
        ON DELETE RESTRICT,   -- preserve messages if user is deactivated

    CONSTRAINT chk_messages_content
        CHECK (LENGTH(TRIM(content)) > 0)
);

-- "fetch last N messages in a channel"
CREATE INDEX idx_messages_channel_created
    ON messages (channel_id, created_at DESC)
    WHERE deleted_at IS NULL;

CREATE TRIGGER trg_channels_updated_at
    BEFORE UPDATE ON channels
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();