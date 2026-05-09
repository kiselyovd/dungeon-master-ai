use app_llm::{ChatMessage, MessagePart, ToolCall, ToolResult};
use app_server::db::{
    init_db, insert_message, journal_insert, journal_list, list_messages_by_session, npc_get,
    npc_get_all, npc_upsert_fact, snapshot_insert, snapshot_load_latest,
};
use sqlx::SqlitePool;
use uuid::Uuid;

async fn in_memory_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite");
    init_db(&pool).await.expect("migrate");
    pool
}

#[tokio::test]
async fn migrations_run_without_error() {
    let _pool = in_memory_pool().await;
    // If we reach here, migrations ran.
}

#[tokio::test]
async fn snapshot_round_trip() {
    let pool = in_memory_pool().await;
    let session_id = Uuid::new_v4();
    let game_state = serde_json::json!({
        "schema_version": 1,
        "state": { "round": 1, "test": true }
    });

    let snap_id = snapshot_insert(&pool, session_id, 1, &game_state, None)
        .await
        .expect("insert snapshot");

    let loaded = snapshot_load_latest(&pool, session_id)
        .await
        .expect("load snapshot")
        .expect("should exist");

    assert_eq!(loaded.id, snap_id);
    assert_eq!(loaded.turn_number, 1);
    assert_eq!(loaded.game_state["state"]["test"], true);
}

#[tokio::test]
async fn snapshot_load_latest_returns_none_when_empty() {
    let pool = in_memory_pool().await;
    let session_id = Uuid::new_v4();
    let result = snapshot_load_latest(&pool, session_id)
        .await
        .expect("query ok");
    assert!(result.is_none());
}

// ---- Journal ----

#[tokio::test]
async fn journal_insert_and_list() {
    let pool = in_memory_pool().await;
    let campaign_id = Uuid::new_v4();
    let entry_id = journal_insert(
        &pool,
        campaign_id,
        "<p>The party entered the dungeon.</p>",
        Some("Chapter 1"),
    )
    .await
    .unwrap();
    let entries = journal_list(&pool, campaign_id).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, entry_id);
    assert_eq!(entries[0].chapter.as_deref(), Some("Chapter 1"));
}

#[tokio::test]
async fn journal_list_empty_for_new_campaign() {
    let pool = in_memory_pool().await;
    let entries = journal_list(&pool, Uuid::new_v4()).await.unwrap();
    assert!(entries.is_empty());
}

#[tokio::test]
async fn journal_entries_ordered_by_creation() {
    let pool = in_memory_pool().await;
    let campaign_id = Uuid::new_v4();
    let id1 = journal_insert(&pool, campaign_id, "<p>First.</p>", None)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let id2 = journal_insert(&pool, campaign_id, "<p>Second.</p>", None)
        .await
        .unwrap();
    let entries = journal_list(&pool, campaign_id).await.unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, id1);
    assert_eq!(entries[1].id, id2);
}

// ---- NPC Memory ----

#[tokio::test]
async fn npc_upsert_creates_new_record() {
    let pool = in_memory_pool().await;
    let campaign_id = Uuid::new_v4();
    npc_upsert_fact(
        &pool,
        campaign_id,
        "Mira",
        "She saved the party in session 2",
        "friendly",
        "innkeeper",
    )
    .await
    .unwrap();
    let npc = npc_get(&pool, campaign_id, "Mira").await.unwrap();
    assert!(npc.is_some());
    let npc = npc.unwrap();
    assert_eq!(npc.name, "Mira");
    assert_eq!(npc.disposition, "friendly");
    assert_eq!(npc.role, "innkeeper");
    assert_eq!(npc.facts.len(), 1);
    assert!(npc.facts[0].text.contains("saved the party"));
}

#[tokio::test]
async fn npc_upsert_appends_facts_to_existing() {
    let pool = in_memory_pool().await;
    let campaign_id = Uuid::new_v4();
    npc_upsert_fact(&pool, campaign_id, "Mira", "First fact", "neutral", "")
        .await
        .unwrap();
    npc_upsert_fact(
        &pool,
        campaign_id,
        "Mira",
        "Second fact",
        "friendly",
        "innkeeper",
    )
    .await
    .unwrap();
    let npc = npc_get(&pool, campaign_id, "Mira").await.unwrap().unwrap();
    assert_eq!(npc.facts.len(), 2);
    assert_eq!(npc.disposition, "friendly"); // updated on second upsert
    assert_eq!(npc.role, "innkeeper");
}

#[tokio::test]
async fn npc_get_all_returns_all_campaign_npcs() {
    let pool = in_memory_pool().await;
    let campaign_id = Uuid::new_v4();
    npc_upsert_fact(&pool, campaign_id, "Mira", "fact a", "friendly", "innkeeper")
        .await
        .unwrap();
    npc_upsert_fact(&pool, campaign_id, "Theron", "fact b", "hostile", "guard")
        .await
        .unwrap();
    let npcs = npc_get_all(&pool, campaign_id).await.unwrap();
    assert_eq!(npcs.len(), 2);
    let names: Vec<&str> = npcs.iter().map(|n| n.name.as_str()).collect();
    assert!(names.contains(&"Mira"));
    assert!(names.contains(&"Theron"));
}

#[tokio::test]
async fn npc_from_different_campaign_not_returned() {
    let pool = in_memory_pool().await;
    let c1 = Uuid::new_v4();
    let c2 = Uuid::new_v4();
    npc_upsert_fact(&pool, c1, "Mira", "fact", "neutral", "")
        .await
        .unwrap();
    let npcs = npc_get_all(&pool, c2).await.unwrap();
    assert!(npcs.is_empty());
}

// ---- M4.5 messages tests ----

#[tokio::test]
async fn insert_message_round_trip_user_with_image() {
    let pool = in_memory_pool().await;
    let session_id = "11111111-1111-1111-1111-111111111111";
    let msg = ChatMessage::User {
        parts: vec![
            MessagePart::Text {
                text: "look".into(),
            },
            MessagePart::Image {
                mime: "image/png".into(),
                data_b64: "aGVsbG8=".into(),
                name: Some("g.png".into()),
            },
        ],
    };
    let id = insert_message(&pool, session_id, &msg).await.unwrap();
    assert_eq!(id.len(), 36);

    let history = list_messages_by_session(&pool, session_id).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0], msg);
}

#[tokio::test]
async fn insert_message_assistant_with_tool_calls_round_trip() {
    let pool = in_memory_pool().await;
    let session_id = "22222222-2222-2222-2222-222222222222";
    let msg = ChatMessage::AssistantWithToolCalls {
        content: Some("rolling".into()),
        tool_calls: vec![ToolCall {
            id: "call_1".into(),
            name: "roll_dice".into(),
            args: serde_json::json!({"d": 20}),
        }],
    };
    insert_message(&pool, session_id, &msg).await.unwrap();
    let history = list_messages_by_session(&pool, session_id).await.unwrap();
    assert_eq!(history[0], msg);
}

#[tokio::test]
async fn insert_message_tool_result_round_trip() {
    let pool = in_memory_pool().await;
    let session_id = "55555555-5555-5555-5555-555555555555";
    let msg = ChatMessage::ToolResult(ToolResult {
        tool_call_id: "call_1".into(),
        content: "{\"rolled\":17}".into(),
        is_error: false,
    });
    insert_message(&pool, session_id, &msg).await.unwrap();
    let history = list_messages_by_session(&pool, session_id).await.unwrap();
    assert_eq!(history[0], msg);
}

#[tokio::test]
async fn list_messages_orders_by_created_at() {
    let pool = in_memory_pool().await;
    let session_id = "33333333-3333-3333-3333-333333333333";
    for i in 0..3 {
        let msg = ChatMessage::user_text(format!("msg {i}"));
        insert_message(&pool, session_id, &msg).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    let history = list_messages_by_session(&pool, session_id).await.unwrap();
    assert_eq!(history.len(), 3);
    let texts: Vec<_> = history
        .iter()
        .filter_map(|m| match m {
            ChatMessage::User { parts } => parts.iter().find_map(|p| match p {
                MessagePart::Text { text } => Some(text.clone()),
                _ => None,
            }),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec!["msg 0", "msg 1", "msg 2"]);
}

#[tokio::test]
async fn list_messages_isolated_per_session() {
    let pool = in_memory_pool().await;
    let s1 = "44444444-4444-4444-4444-444444444444";
    let s2 = "44444444-4444-4444-4444-444444444445";
    insert_message(&pool, s1, &ChatMessage::user_text("a"))
        .await
        .unwrap();
    insert_message(&pool, s2, &ChatMessage::user_text("b"))
        .await
        .unwrap();
    let h1 = list_messages_by_session(&pool, s1).await.unwrap();
    let h2 = list_messages_by_session(&pool, s2).await.unwrap();
    assert_eq!(h1.len(), 1);
    assert_eq!(h2.len(), 1);
}
