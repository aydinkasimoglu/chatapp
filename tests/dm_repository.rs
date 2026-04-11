mod common;

use chatapp::{models::DmConversationKind, repositories::dm::DmRepository};

use common::{insert_user, run_db_test};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::test]
async fn create_conversation_persists_membership_rows() {
    run_db_test(|database| {
        Box::pin(async move {
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_repo_create").await?;
            let bob = insert_user(&database.pool, "bob_repo_create").await?;

            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;
            let participant_ids = repository
                .list_participant_user_ids(conversation.conversation_id)
                .await?;

            assert_eq!(conversation.kind, DmConversationKind::Direct);
            assert_eq!(participant_ids, vec![alice, bob]);
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn get_conversation_access_distinguishes_membership() {
    run_db_test(|database| {
        Box::pin(async move {
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_repo_access").await?;
            let bob = insert_user(&database.pool, "bob_repo_access").await?;
            let carol = insert_user(&database.pool, "carol_repo_access").await?;

            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;

            let member_access = repository
                .get_conversation_access(conversation.conversation_id, alice)
                .await?;
            let non_member_access = repository
                .get_conversation_access(conversation.conversation_id, carol)
                .await?;

            assert!(member_access.conversation_exists);
            assert!(member_access.is_member);
            assert!(non_member_access.conversation_exists);
            assert!(!non_member_access.is_member);
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn list_conversations_orders_by_latest_activity() {
    run_db_test(|database| {
        Box::pin(async move {
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_repo_list").await?;
            let bob = insert_user(&database.pool, "bob_repo_list").await?;
            let carol = insert_user(&database.pool, "carol_repo_list").await?;

            let older = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;
            let newer = repository
                .create_conversation(alice, &[alice, carol], DmConversationKind::Direct, None)
                .await?;
            repository
                .send_message(newer.conversation_id, alice, "most recent")
                .await?;

            let conversations = repository.list_conversations(alice, 10, 0).await?;

            assert_eq!(conversations.len(), 2);
            assert_eq!(conversations[0].conversation_id, newer.conversation_id);
            assert_eq!(conversations[1].conversation_id, older.conversation_id);
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn send_message_persists_a_message_row() {
    run_db_test(|database| {
        Box::pin(async move {
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_repo_send").await?;
            let bob = insert_user(&database.pool, "bob_repo_send").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;

            let message = repository
                .send_message(conversation.conversation_id, alice, "hello repo")
                .await?;
            let persisted = repository
                .find_message_by_id(message.message_id)
                .await?
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "message should exist in repository test",
                    )
                })?;

            assert_eq!(persisted.sender_id, alice);
            assert_eq!(persisted.content, "hello repo");
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn list_messages_honors_before_cursor() {
    run_db_test(|database| {
        Box::pin(async move {
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_repo_cursor").await?;
            let bob = insert_user(&database.pool, "bob_repo_cursor").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;

            let older = repository
                .send_message(conversation.conversation_id, alice, "older")
                .await?;
            let newer = repository
                .send_message(conversation.conversation_id, bob, "newer")
                .await?;

            let first_page = repository
                .list_messages(conversation.conversation_id, None, 1)
                .await?;
            let second_page = repository
                .list_messages(conversation.conversation_id, Some(newer.message_id), 1)
                .await?;

            assert_eq!(first_page.len(), 1);
            assert_eq!(first_page[0].message_id, newer.message_id);
            assert_eq!(second_page.len(), 1);
            assert_eq!(second_page[0].message_id, older.message_id);
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn mark_as_read_updates_the_member_cursor() {
    run_db_test(|database| {
        Box::pin(async move {
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_repo_read").await?;
            let bob = insert_user(&database.pool, "bob_repo_read").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;
            let message = repository
                .send_message(conversation.conversation_id, alice, "read me")
                .await?;

            let updated = repository
                .mark_as_read(conversation.conversation_id, bob, Some(message.message_id))
                .await?;
            let participants = repository
                .list_participants_for_conversations(&[conversation.conversation_id])
                .await?;
            let bob_membership = participants
                .into_iter()
                .find(|participant| participant.user_id == bob)
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "missing bob membership")
                })?;

            assert!(updated);
            assert_eq!(bob_membership.last_read_message_id, Some(message.message_id));
            assert!(bob_membership.last_read_at.is_some());
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn soft_delete_message_sets_deleted_at() {
    run_db_test(|database| {
        Box::pin(async move {
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_repo_delete").await?;
            let bob = insert_user(&database.pool, "bob_repo_delete").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;
            let message = repository
                .send_message(conversation.conversation_id, alice, "delete me")
                .await?;

            let deleted = repository.soft_delete_message(message.message_id).await?;
            let persisted = repository
                .find_message_by_id(message.message_id)
                .await?
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "deleted message missing",
                    )
                })?;

            assert!(deleted);
            assert!(persisted.deleted_at.is_some());
            Ok::<(), BoxError>(())
        })
    })
    .await;
}