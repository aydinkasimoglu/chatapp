-- Rename messages table to channel_messages
-- Also rename all associated constraints and indexes for consistency

ALTER TABLE messages RENAME TO channel_messages;

ALTER INDEX idx_messages_channel_created RENAME TO idx_channel_messages_channel_created;

ALTER TABLE channel_messages RENAME CONSTRAINT fk_messages_channel TO fk_channel_messages_channel;
ALTER TABLE channel_messages RENAME CONSTRAINT fk_messages_user TO fk_channel_messages_user;
ALTER TABLE channel_messages RENAME CONSTRAINT chk_messages_content TO chk_channel_messages_content;
