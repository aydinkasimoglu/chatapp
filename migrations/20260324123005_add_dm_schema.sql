-- =============================================================
-- DIRECT MESSAGES / GROUP DMS
--
-- This migration adds the minimal schema needed for:
-- - 1:1 direct messages
-- - group DMs
-- - per-user read state
-- - per-message reactions
-- =============================================================

-- Distinguishes 1:1 conversations from group chats.
CREATE TYPE dm_conversation_kind AS ENUM ('direct', 'group');


-- One row per DM thread.
CREATE TABLE dm_conversations (
    -- Stable, time-sortable identifier for the conversation.
    conversation_id UUID PRIMARY KEY DEFAULT uuidv7(),

    -- 'direct' for 1:1 chats, 'group' for multi-user chats.
    kind            dm_conversation_kind NOT NULL,

    -- Direct chats derive their display name from members.
    -- Group chats keep a stored title.
    title           VARCHAR(100),

    -- Direct-only user pair columns.
    -- The application should sort the two participant IDs before insert so the
    -- lower UUID goes into direct_user_low_id and the higher UUID goes into
    -- direct_user_high_id.
    direct_user_low_id  UUID,
    direct_user_high_id UUID,

    -- User who originally created the conversation.
    created_by      UUID        NOT NULL,

    -- Audit timestamps.
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_dm_conversations_created_by
        FOREIGN KEY (created_by) REFERENCES users(user_id)
        ON DELETE RESTRICT,

    CONSTRAINT fk_dm_conversations_direct_user_low
        FOREIGN KEY (direct_user_low_id) REFERENCES users(user_id)
        ON DELETE RESTRICT,

    CONSTRAINT fk_dm_conversations_direct_user_high
        FOREIGN KEY (direct_user_high_id) REFERENCES users(user_id)
        ON DELETE RESTRICT,

    -- Direct chats store their participant pair here, must not have a custom
    -- title, and the creator must be one of the two participants.
    -- Group chats must have a title and no direct pair columns.
    CONSTRAINT chk_dm_conversations_shape
        CHECK (
            (
                kind = 'direct'
                AND title IS NULL
                AND direct_user_low_id IS NOT NULL
                AND direct_user_high_id IS NOT NULL
                AND direct_user_low_id < direct_user_high_id
                AND created_by IN (direct_user_low_id, direct_user_high_id)
            )
            OR
            (
                kind = 'group'
                AND direct_user_low_id IS NULL
                AND direct_user_high_id IS NULL
                AND LENGTH(BTRIM(COALESCE(title, ''))) > 0
            )
        )
);

-- Keeps updated_at in sync when conversation metadata changes.
CREATE TRIGGER trg_dm_conversations_updated_at
    BEFORE UPDATE ON dm_conversations
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Hard uniqueness guarantee for 1:1 chats.
-- Because the participant IDs are stored in canonical order, only one direct
-- conversation can exist for a given unordered user pair.
CREATE UNIQUE INDEX uq_dm_conversations_direct_pair
    ON dm_conversations (direct_user_low_id, direct_user_high_id)
    WHERE kind = 'direct';


-- Junction table between conversations and users.
-- This is also where each member's read cursor lives.
CREATE TABLE dm_conversation_members (
    -- Conversation the membership row belongs to.
    conversation_id      UUID        NOT NULL,

    -- User participating in that conversation.
    user_id              UUID        NOT NULL,

    -- When the user joined the conversation.
    joined_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Last message this member has marked as read.
    -- NULL means they have not marked any message as read yet.
    last_read_message_id UUID,

    -- When the read cursor was last updated.
    -- This can still be set when last_read_message_id is NULL,
    -- for example if the conversation was empty when opened.
    last_read_at         TIMESTAMPTZ,

    PRIMARY KEY (conversation_id, user_id),

    CONSTRAINT fk_dm_conversation_members_conversation
        FOREIGN KEY (conversation_id) REFERENCES dm_conversations(conversation_id)
        ON DELETE CASCADE,

    CONSTRAINT fk_dm_conversation_members_user
        FOREIGN KEY (user_id) REFERENCES users(user_id)
        ON DELETE CASCADE,

    -- If a concrete message is marked as read, store when that happened too.
    CONSTRAINT chk_dm_conversation_members_last_read
        CHECK (last_read_message_id IS NULL OR last_read_at IS NOT NULL)
);

-- Fast lookup for "which conversations does this user belong to?"
CREATE INDEX idx_dm_conversation_members_user
    ON dm_conversation_members (user_id, conversation_id);


-- Validates membership invariants that span dm_conversations and
-- dm_conversation_members.
CREATE OR REPLACE FUNCTION validate_dm_conversation_membership(
    p_conversation_id UUID
)
RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE
    conversation_kind dm_conversation_kind;
    direct_low UUID;
    direct_high UUID;
    member_count INTEGER;
    matching_direct_member_count INTEGER;
BEGIN
    IF p_conversation_id IS NULL THEN
        RETURN;
    END IF;

    SELECT
        dc.kind,
        dc.direct_user_low_id,
        dc.direct_user_high_id
    INTO
        conversation_kind,
        direct_low,
        direct_high
    FROM dm_conversations AS dc
    WHERE dc.conversation_id = p_conversation_id;

    -- Conversation may already be gone because of cascading deletes.
    IF NOT FOUND THEN
        RETURN;
    END IF;

    -- Direct conversations must have exactly the two members declared on the
    -- conversation row. No more, no less.
    IF conversation_kind = 'direct' THEN
        SELECT
            COUNT(*),
            COUNT(*) FILTER (
                WHERE dcm.user_id IN (direct_low, direct_high)
            )
        INTO
            member_count,
            matching_direct_member_count
        FROM dm_conversation_members AS dcm
        WHERE dcm.conversation_id = p_conversation_id;

        IF member_count <> 2 OR matching_direct_member_count <> 2 THEN
            RAISE EXCEPTION
                'direct conversation % must have exactly the two direct members',
                p_conversation_id;
        END IF;
    END IF;
END;
$$;


-- Defers membership validation until commit so a transaction can create the
-- conversation row and its membership rows in any order.
CREATE OR REPLACE FUNCTION ensure_dm_conversation_membership_integrity()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP IN ('INSERT', 'UPDATE') THEN
        PERFORM validate_dm_conversation_membership(NEW.conversation_id);
    END IF;

    IF TG_OP IN ('DELETE', 'UPDATE') THEN
        PERFORM validate_dm_conversation_membership(OLD.conversation_id);
    END IF;

    RETURN NULL;
END;
$$;


CREATE CONSTRAINT TRIGGER trg_dm_conversations_validate_membership
    AFTER INSERT OR UPDATE OF kind, direct_user_low_id, direct_user_high_id, created_by ON dm_conversations
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION ensure_dm_conversation_membership_integrity();


CREATE CONSTRAINT TRIGGER trg_dm_conversation_members_validate_membership
    AFTER INSERT OR UPDATE OR DELETE ON dm_conversation_members
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION ensure_dm_conversation_membership_integrity();


-- Message history for DM conversations.
CREATE TABLE dm_messages (
    -- Stable, time-sortable identifier for the message.
    message_id       UUID PRIMARY KEY DEFAULT uuidv7(),

    -- Conversation the message belongs to.
    conversation_id  UUID        NOT NULL,

    -- User who authored the message.
    -- Membership is enforced by a deferred constraint trigger below.
    sender_id        UUID        NOT NULL,

    -- Text body. This schema intentionally supports text only for now.
    content          TEXT        NOT NULL,

    -- Soft-edit / soft-delete timestamps.
    edited_at        TIMESTAMPTZ,
    deleted_at       TIMESTAMPTZ,

    -- Creation timestamp for auditing and fallback ordering.
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Lets other tables safely reference a message together with its
    -- conversation id when cross-conversation safety matters.
    CONSTRAINT uq_dm_messages_conversation_message
        UNIQUE (conversation_id, message_id),

    CONSTRAINT fk_dm_messages_conversation
        FOREIGN KEY (conversation_id) REFERENCES dm_conversations(conversation_id)
        ON DELETE CASCADE,

    CONSTRAINT fk_dm_messages_sender
        FOREIGN KEY (sender_id) REFERENCES users(user_id)
        ON DELETE RESTRICT,

    -- Prevent blank or whitespace-only messages.
    CONSTRAINT chk_dm_messages_content
        CHECK (LENGTH(BTRIM(content)) > 0),

    -- Prevent edit/delete timestamps from predating the original insert time.
    CONSTRAINT chk_dm_messages_timestamps
        CHECK (
            (edited_at IS NULL OR edited_at >= created_at)
            AND
            (deleted_at IS NULL OR deleted_at >= created_at)
        )
);

CREATE INDEX idx_dm_messages_active_history
    ON dm_messages (conversation_id, message_id DESC)
    WHERE deleted_at IS NULL;


-- Validates that a message sender belongs to the conversation at write time.
CREATE OR REPLACE FUNCTION ensure_dm_message_sender_membership()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM dm_conversation_members AS dcm
        WHERE dcm.conversation_id = NEW.conversation_id
          AND dcm.user_id = NEW.sender_id
    ) THEN
        RAISE EXCEPTION
            'sender % must be a member of conversation %',
            NEW.sender_id,
            NEW.conversation_id;
    END IF;

    RETURN NULL;
END;
$$;


CREATE CONSTRAINT TRIGGER trg_dm_messages_validate_sender_membership
    AFTER INSERT OR UPDATE OF conversation_id, sender_id ON dm_messages
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION ensure_dm_message_sender_membership();


-- Keep the read cursor conversation-safe.
-- A member can only point at a message from the same conversation.
ALTER TABLE dm_conversation_members
    ADD CONSTRAINT fk_dm_conversation_members_last_read_message
    FOREIGN KEY (conversation_id, last_read_message_id)
    REFERENCES dm_messages (conversation_id, message_id)
    ON DELETE SET NULL (last_read_message_id);


-- One row per user/reaction on a message.
CREATE TABLE dm_message_reactions (
    -- Message receiving the reaction.
    message_id  UUID        NOT NULL,

    -- User who added the reaction.
    -- Membership is enforced by a deferred constraint trigger below.
    user_id     UUID        NOT NULL,

    -- Reaction token, such as an emoji or short code.
    reaction    VARCHAR(32) NOT NULL,

    -- When the reaction was created.
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Prevent duplicate reactions of the same type by the same user.
    PRIMARY KEY (message_id, user_id, reaction),

    CONSTRAINT fk_dm_message_reactions_message
        FOREIGN KEY (message_id) REFERENCES dm_messages(message_id)
        ON DELETE CASCADE,

    CONSTRAINT fk_dm_message_reactions_user
        FOREIGN KEY (user_id) REFERENCES users(user_id)
        ON DELETE CASCADE,

    -- Prevent empty or whitespace-only reaction values.
    CONSTRAINT chk_dm_message_reactions_value
        CHECK (LENGTH(BTRIM(reaction)) > 0)
);

-- Speeds up "show conversations created by this user" or ownership checks.
CREATE INDEX idx_dm_conversations_created_by
    ON dm_conversations (created_by);

-- Speeds up "show all messages sent by this user" queries and moderation tools.
CREATE INDEX idx_dm_messages_sender_id
    ON dm_messages (sender_id);

-- Helps both application queries and FK maintenance for the read pointer.
CREATE INDEX idx_dm_conversation_members_last_read
    ON dm_conversation_members (last_read_message_id);

-- The PK is (message_id, user_id, reaction), so user_id alone needs its own
-- index for user-centric lookups and FK maintenance.
CREATE INDEX idx_dm_message_reactions_user_id
    ON dm_message_reactions (user_id);


-- Validates that a reacting user belongs to the message's conversation.
CREATE OR REPLACE FUNCTION ensure_dm_message_reaction_membership()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    reaction_conversation_id UUID;
BEGIN
    SELECT dm.conversation_id
    INTO reaction_conversation_id
    FROM dm_messages AS dm
    WHERE dm.message_id = NEW.message_id;

    -- The FK to dm_messages will report a clearer error if the message is gone.
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM dm_conversation_members AS dcm
        WHERE dcm.conversation_id = reaction_conversation_id
          AND dcm.user_id = NEW.user_id
    ) THEN
        RAISE EXCEPTION
            'reaction user % must be a member of conversation % for message %',
            NEW.user_id,
            reaction_conversation_id,
            NEW.message_id;
    END IF;

    RETURN NULL;
END;
$$;


CREATE CONSTRAINT TRIGGER trg_dm_message_reactions_validate_membership
    AFTER INSERT OR UPDATE OF message_id, user_id ON dm_message_reactions
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION ensure_dm_message_reaction_membership();