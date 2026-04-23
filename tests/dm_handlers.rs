mod common;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use chatapp::{
    error::ServiceError,
    extractors::AuthenticatedUser,
    handlers::dms::{
        create_conversation_handler, delete_message_handler, get_conversation_handler,
        list_conversations_handler, list_messages_handler, mark_as_read_handler,
        send_message_handler,
    },
    models::{
        CreateDmConversation, DmConversationKind, DmConversationListQuery, DmMessageListQuery,
        MarkDmConversationRead, SendDmMessage,
    },
    repositories::dm::DmRepository,
};

use common::{build_test_state, insert_user, run_db_test};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::test]
async fn create_conversation_handler_returns_created_for_new_direct_conversation() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let alice = insert_user(&database.pool, "alice_handler_create_ok").await?;
            let bob = insert_user(&database.pool, "bob_handler_create_ok").await?;

            let (status, Json(conversation)) = create_conversation_handler(
                State(state),
                AuthenticatedUser { user_id: alice },
                Json(CreateDmConversation {
                    participant_ids: vec![bob],
                    title: None,
                }),
            )
            .await?;

            assert_eq!(status, StatusCode::CREATED);
            assert_eq!(conversation.kind, DmConversationKind::Direct);
            assert_eq!(conversation.participant_count, 2);
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn create_conversation_handler_returns_validation_error_for_group_without_title() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let alice = insert_user(&database.pool, "alice_handler_create_err").await?;
            let bob = insert_user(&database.pool, "bob_handler_create_err").await?;
            let carol = insert_user(&database.pool, "carol_handler_create_err").await?;

            let error = create_conversation_handler(
                State(state),
                AuthenticatedUser { user_id: alice },
                Json(CreateDmConversation {
                    participant_ids: vec![bob, carol],
                    title: None,
                }),
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
async fn list_conversations_handler_returns_paginated_items() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let alice = insert_user(&database.pool, "alice_handler_list_conv_ok").await?;
            let bob = insert_user(&database.pool, "bob_handler_list_conv_ok").await?;
            state
                .dm_service
                .create_conversation(
                    alice,
                    CreateDmConversation {
                        participant_ids: vec![bob],
                        title: None,
                    },
                )
                .await?;

            let Json(response) = list_conversations_handler(
                State(state),
                AuthenticatedUser { user_id: alice },
                Query(DmConversationListQuery {
                    limit: Some(10),
                    offset: Some(0),
                }),
            )
            .await?;

            assert_eq!(response.items.len(), 1);
            assert_eq!(response.limit, 10);
            assert_eq!(response.offset, 0);
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn list_conversations_handler_rejects_invalid_page_size() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let alice = insert_user(&database.pool, "alice_handler_list_conv_err").await?;

            let error = list_conversations_handler(
                State(state),
                AuthenticatedUser { user_id: alice },
                Query(DmConversationListQuery {
                    limit: Some(0),
                    offset: Some(0),
                }),
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
async fn get_conversation_handler_returns_the_requested_conversation() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let alice = insert_user(&database.pool, "alice_handler_get_ok").await?;
            let bob = insert_user(&database.pool, "bob_handler_get_ok").await?;
            let (conversation, _) = state
                .dm_service
                .create_conversation(
                    alice,
                    CreateDmConversation {
                        participant_ids: vec![bob],
                        title: None,
                    },
                )
                .await?;

            let Json(fetched) = get_conversation_handler(
                State(state),
                AuthenticatedUser { user_id: alice },
                Path(conversation.conversation_id),
            )
            .await?;

            assert_eq!(fetched.conversation_id, conversation.conversation_id);
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn get_conversation_handler_forbids_non_members() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_handler_get_err").await?;
            let bob = insert_user(&database.pool, "bob_handler_get_err").await?;
            let carol = insert_user(&database.pool, "carol_handler_get_err").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;

            let error = get_conversation_handler(
                State(state),
                AuthenticatedUser { user_id: carol },
                Path(conversation.conversation_id),
            )
            .await
            .unwrap_err();

            assert!(matches!(error, ServiceError::Forbidden));
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn send_message_handler_returns_created_message() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_handler_send_ok").await?;
            let bob = insert_user(&database.pool, "bob_handler_send_ok").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;

            let (status, Json(message)) = send_message_handler(
                State(state),
                AuthenticatedUser { user_id: alice },
                Path(conversation.conversation_id),
                Json(SendDmMessage {
                    content: "hello handler".to_string(),
                }),
            )
            .await?;

            assert_eq!(status, StatusCode::CREATED);
            assert_eq!(message.content.as_deref(), Some("hello handler"));
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn send_message_handler_rejects_blank_messages() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_handler_send_err").await?;
            let bob = insert_user(&database.pool, "bob_handler_send_err").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;

            let error = send_message_handler(
                State(state),
                AuthenticatedUser { user_id: alice },
                Path(conversation.conversation_id),
                Json(SendDmMessage {
                    content: "   ".to_string(),
                }),
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
async fn list_messages_handler_returns_a_cursor_page() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_handler_list_msg_ok").await?;
            let bob = insert_user(&database.pool, "bob_handler_list_msg_ok").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;
            repository
                .send_message(conversation.conversation_id, alice, "hello page")
                .await?;

            let Json(page) = list_messages_handler(
                State(state),
                AuthenticatedUser { user_id: alice },
                Path(conversation.conversation_id),
                Query(DmMessageListQuery {
                    limit: Some(10),
                    before_message_id: None,
                }),
            )
            .await?;

            assert_eq!(page.items.len(), 1);
            assert_eq!(page.limit, 10);
            assert!(!page.has_older);
            assert!(page.next_before_message_id.is_none());
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn list_messages_handler_sets_has_older_when_another_page_exists() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_handler_list_msg_page").await?;
            let bob = insert_user(&database.pool, "bob_handler_list_msg_page").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;
            repository
                .send_message(conversation.conversation_id, alice, "older page item")
                .await?;
            let newest = repository
                .send_message(conversation.conversation_id, bob, "newest page item")
                .await?;

            let Json(page) = list_messages_handler(
                State(state),
                AuthenticatedUser { user_id: alice },
                Path(conversation.conversation_id),
                Query(DmMessageListQuery {
                    limit: Some(1),
                    before_message_id: None,
                }),
            )
            .await?;

            assert_eq!(page.items.len(), 1);
            assert!(page.has_older);
            assert_eq!(page.items[0].message_id, newest.message_id);
            assert_eq!(page.next_before_message_id, Some(newest.message_id));
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn list_messages_handler_rejects_invalid_page_size() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let alice = insert_user(&database.pool, "alice_handler_list_msg_err").await?;

            let error = list_messages_handler(
                State(state),
                AuthenticatedUser { user_id: alice },
                Path(Uuid::new_v4()),
                Query(DmMessageListQuery {
                    limit: Some(0),
                    before_message_id: None,
                }),
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
async fn mark_as_read_handler_returns_no_content() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_handler_mark_ok").await?;
            let bob = insert_user(&database.pool, "bob_handler_mark_ok").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;
            let message = repository
                .send_message(conversation.conversation_id, alice, "read it")
                .await?;

            let status = mark_as_read_handler(
                State(state),
                AuthenticatedUser { user_id: bob },
                Path(conversation.conversation_id),
                Json(MarkDmConversationRead {
                    up_to_message_id: Some(message.message_id),
                }),
            )
            .await?;

            assert_eq!(status, StatusCode::NO_CONTENT);
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn mark_as_read_handler_rejects_foreign_message_ids() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_handler_mark_err").await?;
            let bob = insert_user(&database.pool, "bob_handler_mark_err").await?;
            let carol = insert_user(&database.pool, "carol_handler_mark_err").await?;
            let first_conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;
            let second_conversation = repository
                .create_conversation(alice, &[alice, carol], DmConversationKind::Direct, None)
                .await?;
            let foreign_message = repository
                .send_message(second_conversation.conversation_id, alice, "foreign")
                .await?;

            let error = mark_as_read_handler(
                State(state),
                AuthenticatedUser { user_id: bob },
                Path(first_conversation.conversation_id),
                Json(MarkDmConversationRead {
                    up_to_message_id: Some(foreign_message.message_id),
                }),
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
async fn delete_message_handler_returns_no_content_for_the_author() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_handler_delete_ok").await?;
            let bob = insert_user(&database.pool, "bob_handler_delete_ok").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;
            let message = repository
                .send_message(conversation.conversation_id, alice, "bye")
                .await?;

            let status = delete_message_handler(
                State(state),
                AuthenticatedUser { user_id: alice },
                Path(message.message_id),
            )
            .await?;

            assert_eq!(status, StatusCode::NO_CONTENT);
            Ok::<(), BoxError>(())
        })
    })
    .await;
}

#[tokio::test]
async fn delete_message_handler_forbids_non_authors() {
    run_db_test(|database| {
        Box::pin(async move {
            let state = build_test_state(database.pool.clone())?;
            let repository = DmRepository::new(database.pool.clone());
            let alice = insert_user(&database.pool, "alice_handler_delete_err").await?;
            let bob = insert_user(&database.pool, "bob_handler_delete_err").await?;
            let conversation = repository
                .create_conversation(alice, &[alice, bob], DmConversationKind::Direct, None)
                .await?;
            let message = repository
                .send_message(conversation.conversation_id, alice, "cannot delete")
                .await?;

            let error = delete_message_handler(
                State(state),
                AuthenticatedUser { user_id: bob },
                Path(message.message_id),
            )
            .await
            .unwrap_err();

            assert!(matches!(error, ServiceError::Forbidden));
            Ok::<(), BoxError>(())
        })
    })
    .await;
}
