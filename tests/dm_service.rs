mod common;

use chatapp::{
    error::ServiceError,
    models::{CreateDmConversation, DmConversationKind},
    repositories::dm::DmRepository,
};

use common::{build_test_state, insert_block, insert_user, run_db_test};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::test]
async fn create_conversation_reuses_an_existing_direct_thread() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let alice = insert_user(&database.pool, "alice_service_existing").await?;
            let bob = insert_user(&database.pool, "bob_service_existing").await?;

            let (_, first_created) = state
                .dm_service
                .create_conversation(
                    alice,
                    CreateDmConversation {
                        participant_ids: vec![bob],
                        title: None,
                    },
                )
                .await?;
            let (second_conversation, second_created) = state
                .dm_service
                .create_conversation(
                    bob,
                    CreateDmConversation {
                        participant_ids: vec![alice],
                        title: None,
                    },
                )
                .await?;

            assert!(first_created);
            assert!(!second_created);
            assert_eq!(second_conversation.kind, DmConversationKind::Direct);
            assert_eq!(second_conversation.participant_count, 2);
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn create_conversation_requires_a_group_title() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let alice = insert_user(&database.pool, "alice_service_group_title").await?;
            let bob = insert_user(&database.pool, "bob_service_group_title").await?;
            let carol = insert_user(&database.pool, "carol_service_group_title").await?;

            let error: ServiceError = state
                .dm_service
                .create_conversation(
                    alice,
                    CreateDmConversation {
                        participant_ids: vec![bob, carol],
                        title: None,
                    },
                )
                .await
                .unwrap_err();

            assert!(matches!(error, ServiceError::ValidationError(_)));
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn create_conversation_rejects_blocked_pairs() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let alice = insert_user(&database.pool, "alice_service_blocked").await?;
            let bob = insert_user(&database.pool, "bob_service_blocked").await?;
            insert_block(&database.pool, alice, bob).await?;

            let error: ServiceError = state
                .dm_service
                .create_conversation(
                    alice,
                    CreateDmConversation {
                        participant_ids: vec![bob],
                        title: None,
                    },
                )
                .await
                .unwrap_err();

            assert!(matches!(error, ServiceError::ValidationError(_)));
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn get_conversation_forbids_non_members() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_service_forbidden_get").await?;
            let bob = insert_user(&database.pool, "bob_service_forbidden_get").await?;
            let carol = insert_user(&database.pool, "carol_service_forbidden_get").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;

            let error: ServiceError = state
                .dm_service
                .get_conversation(conversation.conversation_id, carol)
                .await
                .unwrap_err();

            assert!(matches!(error, ServiceError::Forbidden));
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn send_message_rejects_non_members() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_service_forbidden_send").await?;
            let bob = insert_user(&database.pool, "bob_service_forbidden_send").await?;
            let carol = insert_user(&database.pool, "carol_service_forbidden_send").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;

            let error: ServiceError = state
                .dm_service
                .send_message(conversation.conversation_id, carol, "no access".to_string())
                .await
                .unwrap_err();

            assert!(matches!(error, ServiceError::Forbidden));
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn send_message_rejects_blank_content() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_service_blank_send").await?;
            let bob = insert_user(&database.pool, "bob_service_blank_send").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;

            let error: ServiceError = state
                .dm_service
                .send_message(conversation.conversation_id, alice, "   ".to_string())
                .await
                .unwrap_err();

            assert!(matches!(error, ServiceError::ValidationError(_)));
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn mark_as_read_requires_a_message_from_the_same_conversation() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_service_mark_read").await?;
            let bob = insert_user(&database.pool, "bob_service_mark_read").await?;
            let carol = insert_user(&database.pool, "carol_service_mark_read").await?;
            let first_conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;
            let second_conversation = repository
                .create_conversation(alice, &[alice, carol], DmConversationKind::Direct, None)
                .await?;
            let foreign_message = repository
                .send_message(second_conversation.conversation_id, alice, "foreign")
                .await?;

            let error: ServiceError = state
                .dm_service
                .mark_as_read(
                    first_conversation.conversation_id,
                    bob,
                    Some(foreign_message.message_id),
                )
                .await
                .unwrap_err();

            assert!(matches!(error, ServiceError::NotFound));
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn delete_message_requires_the_message_author() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_service_delete_author").await?;
            let bob = insert_user(&database.pool, "bob_service_delete_author").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;
            let message = repository
                .send_message(conversation.conversation_id, alice, "protected")
                .await?;

            let error: ServiceError = state
                .dm_service
                .delete_message(message.message_id, bob)
                .await
                .unwrap_err();

            assert!(matches!(error, ServiceError::Forbidden));
            Ok::<(), BoxError>(())
        })
    })
    .await;
}