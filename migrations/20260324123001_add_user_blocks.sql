CREATE TABLE user_blocks (
    block_id   UUID        PRIMARY KEY DEFAULT uuidv7(),
    blocker_id UUID        NOT NULL,
    blocked_id UUID        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_ub_blocker
        FOREIGN KEY (blocker_id) REFERENCES users(user_id) ON DELETE CASCADE,
    CONSTRAINT fk_ub_blocked
        FOREIGN KEY (blocked_id) REFERENCES users(user_id) ON DELETE CASCADE,
    CONSTRAINT chk_ub_distinct
        CHECK (blocker_id <> blocked_id),
    CONSTRAINT uq_ub_pair
        UNIQUE (blocker_id, blocked_id)
);

-- Fast lookup: "list all users that blocker_id has blocked"
CREATE INDEX idx_user_blocks_blocker ON user_blocks (blocker_id);
-- Fast lookup: "has anyone blocked blocked_id?" (used in send_request guard)
CREATE INDEX idx_user_blocks_blocked ON user_blocks (blocked_id);
