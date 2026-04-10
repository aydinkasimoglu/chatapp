-- =============================================================
-- ENFORCE GROUP DM MEMBER COUNTS
--
-- Follow-up migration for the DM schema. The original migration enforced
-- exact membership for direct conversations but left group conversations
-- unconstrained. This migration requires group conversations to keep at
-- least two members and supports an optional database-level maximum via the
-- custom setting app.dm_group_max_members.
-- =============================================================

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
    configured_group_max_members TEXT;
    max_group_members INTEGER;
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

    SELECT
        COUNT(*)
    INTO member_count
    FROM dm_conversation_members AS dcm
    WHERE dcm.conversation_id = p_conversation_id;

    -- Direct conversations must have exactly the two members declared on the
    -- conversation row. No more, no less.
    IF conversation_kind = 'direct' THEN
        SELECT
            COUNT(*) FILTER (
                WHERE dcm.user_id IN (direct_low, direct_high)
            )
        INTO matching_direct_member_count
        FROM dm_conversation_members AS dcm
        WHERE dcm.conversation_id = p_conversation_id;

        IF member_count <> 2 OR matching_direct_member_count <> 2 THEN
            RAISE EXCEPTION
                'direct conversation % must have exactly the two direct members',
                p_conversation_id;
        END IF;

        RETURN;
    END IF;

    -- Group conversations must keep at least two members. An optional maximum
    -- can be configured with:
    --   ALTER DATABASE <db_name> SET app.dm_group_max_members = ''25'';
    IF member_count < 2 THEN
        RAISE EXCEPTION
            'group conversation % must have at least 2 members',
            p_conversation_id;
    END IF;

    configured_group_max_members := NULLIF(
        current_setting('app.dm_group_max_members', true),
        ''
    );

    IF configured_group_max_members IS NULL THEN
        RETURN;
    END IF;

    IF configured_group_max_members !~ '^[0-9]+$' THEN
        RAISE EXCEPTION
            'app.dm_group_max_members must be a positive integer, got %',
            configured_group_max_members;
    END IF;

    max_group_members := configured_group_max_members::INTEGER;

    IF max_group_members < 2 THEN
        RAISE EXCEPTION
            'app.dm_group_max_members must be at least 2, got %',
            max_group_members;
    END IF;

    IF member_count > max_group_members THEN
        RAISE EXCEPTION
            'group conversation % cannot exceed % members',
            p_conversation_id,
            max_group_members;
    END IF;
END;
$$;



-- Validate existing conversations immediately so pre-existing invalid rows do
-- not remain in the database after this migration lands.
DO $$
DECLARE
    conversation_record RECORD;
BEGIN
    FOR conversation_record IN
        SELECT dc.conversation_id
        FROM dm_conversations AS dc
    LOOP
        PERFORM validate_dm_conversation_membership(conversation_record.conversation_id);
    END LOOP;
END;
$$;