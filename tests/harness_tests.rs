use sts2_rs::logger::GameLogger;
use sts2_rs::protocol::Command;
use tempfile::tempdir;

#[test]
fn test_game_logger_jsonl() {
    let tmp = tempdir().unwrap();
    let mut logger = GameLogger::new(tmp.path(), "Ironclad", "test_seed", true);

    let state_val = serde_json::json!({
        "msg_type": "state",
        "player": {
            "hp": 80,
            "energy": 3
        }
    });

    let action_val = serde_json::json!({
        "cmd": "action",
        "action": "end_turn"
    });

    logger.log_state(&state_val);
    logger.log_action(&action_val);

    let log_path = logger.path().unwrap();
    assert!(log_path.exists());
    let content = std::fs::read_to_string(log_path).unwrap();
    assert!(content.contains("\"type\":\"state\""));
    assert!(content.contains("\"type\":\"action\""));
}

#[test]
fn test_protocol_command_serialization() {
    let play = Command::play_card(0, Some(1));
    let json = serde_json::to_string(&play).unwrap();
    assert!(json.contains("play_card"));
    assert!(json.contains("\"card_index\":0"));
    assert!(json.contains("\"target_index\":1"));

    let end_turn = Command::end_turn();
    let json = serde_json::to_string(&end_turn).unwrap();
    assert!(json.contains("end_turn"));
}
