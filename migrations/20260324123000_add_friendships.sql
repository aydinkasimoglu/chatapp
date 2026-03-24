CREATE TYPE friendship_status AS ENUM ('pending', 'accepted', 'rejected');

CREATE TABLE friendships (
    friendship_id UUID PRIMARY KEY DEFAULT uuidv7(),
    requester_id  UUID NOT NULL,
    addressee_id  UUID NOT NULL,
    status        friendship_status NOT NULL DEFAULT 'pending',
    responded_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_friendships_requester
        FOREIGN KEY (requester_id) REFERENCES users(user_id) ON DELETE CASCADE,
    CONSTRAINT fk_friendships_addressee
        FOREIGN KEY (addressee_id) REFERENCES users(user_id) ON DELETE CASCADE,
    CONSTRAINT chk_friendships_distinct_users
        CHECK (requester_id <> addressee_id)
);

CREATE UNIQUE INDEX uq_friendships_user_pair
    ON friendships (
        LEAST(requester_id, addressee_id),
        GREATEST(requester_id, addressee_id)
    );

CREATE INDEX idx_friendships_requester_pending
    ON friendships (requester_id, created_at DESC)
    WHERE status = 'pending';

CREATE INDEX idx_friendships_addressee_pending
    ON friendships (addressee_id, created_at DESC)
    WHERE status = 'pending';

CREATE INDEX idx_friendships_user_status
    ON friendships (requester_id, addressee_id, status);

CREATE TRIGGER trg_friendships_updated_at
    BEFORE UPDATE ON friendships
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
