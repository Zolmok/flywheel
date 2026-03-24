use super::*;
use std::collections::HashMap;

// ── merge_config: CLI flags produce correct Config ──────────────────

#[test]
fn merge_config_cli_flags_produce_correct_config() {
    let file = FileConfig::default();
    let cli = Cli {
        project: Some(42),
        owner: Some("acme".to_string()),
        max_cycles: 3,
        batch_size: 10,
    };

    let result = merge_config(file, &cli);
    match result {
        Ok(config) => {
            assert_eq!(config.project, 42);
            assert_eq!(config.owner, "acme");
            assert_eq!(config.max_cycles, 3);
            assert_eq!(config.batch_size, 10);
        }
        Err(e) => panic!("expected Ok, got Err: {e}"),
    }
}

// ── merge_config: missing project errors ────────────────────────────

#[test]
fn merge_config_missing_project_returns_error() {
    let file = FileConfig::default();
    let cli = Cli {
        project: None,
        owner: Some("acme".to_string()),
        max_cycles: 0,
        batch_size: 5,
    };

    let result = merge_config(file, &cli);
    match result {
        Ok(_) => panic!("expected Err, got Ok"),
        Err(e) => assert!(e.contains("project")),
    }
}

// ── merge_config: missing owner errors ──────────────────────────────

#[test]
fn merge_config_missing_owner_returns_error() {
    let file = FileConfig::default();
    let cli = Cli {
        project: Some(1),
        owner: None,
        max_cycles: 0,
        batch_size: 5,
    };

    let result = merge_config(file, &cli);
    match result {
        Ok(_) => panic!("expected Err, got Ok"),
        Err(e) => assert!(e.contains("owner")),
    }
}

// ── merge_config: file config works alone ───────────────────────────

#[test]
fn merge_config_file_config_works_alone() {
    let file = FileConfig {
        project: Some(99),
        owner: Some("file-owner".to_string()),
    };
    let cli = Cli {
        project: None,
        owner: None,
        max_cycles: 0,
        batch_size: 5,
    };

    let result = merge_config(file, &cli);
    match result {
        Ok(config) => {
            assert_eq!(config.project, 99);
            assert_eq!(config.owner, "file-owner");
        }
        Err(e) => panic!("expected Ok, got Err: {e}"),
    }
}

// ── merge_config: CLI overrides file config ─────────────────────────

#[test]
fn merge_config_cli_overrides_file_config() {
    let file = FileConfig {
        project: Some(10),
        owner: Some("file-owner".to_string()),
    };
    let cli = Cli {
        project: Some(20),
        owner: Some("cli-owner".to_string()),
        max_cycles: 7,
        batch_size: 15,
    };

    let result = merge_config(file, &cli);
    match result {
        Ok(config) => {
            assert_eq!(config.project, 20);
            assert_eq!(config.owner, "cli-owner");
            assert_eq!(config.max_cycles, 7);
            assert_eq!(config.batch_size, 15);
        }
        Err(e) => panic!("expected Ok, got Err: {e}"),
    }
}

// ── next_phase: all 6 transitions ───────────────────────────────────

#[test]
fn next_phase_generate_tickets_to_size_prioritize() {
    assert_eq!(
        next_phase(&Phase::GenerateTickets, false),
        Phase::SizePrioritize
    );
}

#[test]
fn next_phase_size_prioritize_to_move_to_ready() {
    assert_eq!(
        next_phase(&Phase::SizePrioritize, false),
        Phase::MoveToReady
    );
}

#[test]
fn next_phase_move_to_ready_to_implement_ticket() {
    assert_eq!(
        next_phase(&Phase::MoveToReady, false),
        Phase::ImplementTicket
    );
}

#[test]
fn next_phase_implement_ticket_to_check_ready() {
    assert_eq!(
        next_phase(&Phase::ImplementTicket, false),
        Phase::CheckReady
    );
}

#[test]
fn next_phase_check_ready_with_items_to_implement_ticket() {
    assert_eq!(next_phase(&Phase::CheckReady, true), Phase::ImplementTicket);
}

#[test]
fn next_phase_check_ready_without_items_to_generate_tickets() {
    assert_eq!(
        next_phase(&Phase::CheckReady, false),
        Phase::GenerateTickets
    );
}

// ── Phase Display ───────────────────────────────────────────────────

#[test]
fn phase_display_generate_tickets() {
    assert_eq!(format!("{}", Phase::GenerateTickets), "Generate Tickets");
}

#[test]
fn phase_display_size_prioritize() {
    assert_eq!(format!("{}", Phase::SizePrioritize), "Size & Prioritize");
}

#[test]
fn phase_display_move_to_ready() {
    assert_eq!(format!("{}", Phase::MoveToReady), "Move to Ready");
}

#[test]
fn phase_display_implement_ticket() {
    assert_eq!(format!("{}", Phase::ImplementTicket), "Implement Ticket");
}

#[test]
fn phase_display_check_ready() {
    assert_eq!(format!("{}", Phase::CheckReady), "Check Ready");
}

// ── spawn_and_capture ────────────────────────────────────────────────

#[test]
fn spawn_and_capture_echo_returns_output() {
    let result = spawn_and_capture("test", "echo", &["hello"], &HashMap::new());
    match result {
        Some(output) => assert_eq!(output, "hello\n"),
        None => panic!("expected Some, got None"),
    }
}

#[test]
fn spawn_and_capture_nonexistent_program_returns_none() {
    let result = spawn_and_capture("test", "nonexistent_program_xyz", &[], &HashMap::new());
    assert!(result.is_none(), "expected None, got Some");
}

#[test]
fn spawn_and_capture_captures_multiline_output() {
    let result = spawn_and_capture(
        "test",
        "printf",
        &["line1\nline2\nline3\n"],
        &HashMap::new(),
    );
    match result {
        Some(output) => {
            assert!(output.contains("line1"));
            assert!(output.contains("line2"));
            assert!(output.contains("line3"));
        }
        None => panic!("expected Some, got None"),
    }
}

#[test]
fn spawn_and_capture_failed_exit_still_returns_output() {
    let result = spawn_and_capture(
        "test",
        "sh",
        &["-c", "echo output && exit 1"],
        &HashMap::new(),
    );
    match result {
        Some(output) => {
            assert!(output.contains("output"));
        }
        None => panic!("expected Some, got None"),
    }
}

// ── build_generate_tickets_prompt ───────────────────────────────────

#[test]
fn build_generate_tickets_prompt_contains_project_number() {
    let config = Config {
        project: 42,
        owner: "acme".to_string(),
        max_cycles: 0,
        batch_size: 5,
    };
    let prompt = build_generate_tickets_prompt(&config);
    assert!(
        prompt.contains("42"),
        "prompt should contain project number"
    );
}

#[test]
fn build_generate_tickets_prompt_contains_owner() {
    let config = Config {
        project: 42,
        owner: "acme".to_string(),
        max_cycles: 0,
        batch_size: 5,
    };
    let prompt = build_generate_tickets_prompt(&config);
    assert!(prompt.contains("acme"), "prompt should contain owner");
}

#[test]
fn build_generate_tickets_prompt_contains_generate_tickets_skill() {
    let config = Config {
        project: 1,
        owner: "org".to_string(),
        max_cycles: 0,
        batch_size: 5,
    };
    let prompt = build_generate_tickets_prompt(&config);
    assert!(
        prompt.contains("generate-tickets"),
        "prompt should contain generate-tickets skill name"
    );
}

// ── build_size_prioritize_prompt ────────────────────────────────────

#[test]
fn build_size_prioritize_prompt_contains_project_number() {
    let config = Config {
        project: 77,
        owner: "widgets".to_string(),
        max_cycles: 0,
        batch_size: 5,
    };
    let prompt = build_size_prioritize_prompt(&config);
    assert!(
        prompt.contains("77"),
        "prompt should contain project number"
    );
}

#[test]
fn build_size_prioritize_prompt_contains_owner() {
    let config = Config {
        project: 77,
        owner: "widgets".to_string(),
        max_cycles: 0,
        batch_size: 5,
    };
    let prompt = build_size_prioritize_prompt(&config);
    assert!(prompt.contains("widgets"), "prompt should contain owner");
}

// ── build_move_to_ready_prompt ──────────────────────────────────────

#[test]
fn build_move_to_ready_prompt_contains_project_number() {
    let config = Config {
        project: 55,
        owner: "team".to_string(),
        max_cycles: 0,
        batch_size: 8,
    };
    let prompt = build_move_to_ready_prompt(&config);
    assert!(
        prompt.contains("55"),
        "prompt should contain project number"
    );
}

#[test]
fn build_move_to_ready_prompt_contains_owner() {
    let config = Config {
        project: 55,
        owner: "team".to_string(),
        max_cycles: 0,
        batch_size: 8,
    };
    let prompt = build_move_to_ready_prompt(&config);
    assert!(prompt.contains("team"), "prompt should contain owner");
}

#[test]
fn build_move_to_ready_prompt_contains_batch_size() {
    let config = Config {
        project: 55,
        owner: "team".to_string(),
        max_cycles: 0,
        batch_size: 8,
    };
    let prompt = build_move_to_ready_prompt(&config);
    assert!(prompt.contains("8"), "prompt should contain batch_size");
}

// ── build_implement_ticket_prompt ───────────────────────────────────

#[test]
fn build_implement_ticket_prompt_contains_project_number() {
    let config = Config {
        project: 33,
        owner: "dev".to_string(),
        max_cycles: 0,
        batch_size: 5,
    };
    let prompt = build_implement_ticket_prompt(&config);
    assert!(
        prompt.contains("33"),
        "prompt should contain project number"
    );
}

#[test]
fn build_implement_ticket_prompt_contains_owner() {
    let config = Config {
        project: 33,
        owner: "dev".to_string(),
        max_cycles: 0,
        batch_size: 5,
    };
    let prompt = build_implement_ticket_prompt(&config);
    assert!(prompt.contains("dev"), "prompt should contain owner");
}

#[test]
fn build_implement_ticket_prompt_contains_implement_ticket_skill() {
    let config = Config {
        project: 1,
        owner: "org".to_string(),
        max_cycles: 0,
        batch_size: 5,
    };
    let prompt = build_implement_ticket_prompt(&config);
    assert!(
        prompt.contains("implement-ticket"),
        "prompt should contain implement-ticket skill name"
    );
}

// ── parse_ready_items ───────────────────────────────────────────────

#[test]
fn parse_ready_items_returns_true_when_ready_item_exists() {
    let json = r#"{"items":[{"status":"Ready","title":"Do something"}],"totalCount":1}"#;
    assert!(parse_ready_items(json));
}

#[test]
fn parse_ready_items_returns_true_with_mixed_statuses() {
    let json = r#"{"items":[{"status":"Backlog","title":"A"},{"status":"Ready","title":"B"}],"totalCount":2}"#;
    assert!(parse_ready_items(json));
}

#[test]
fn parse_ready_items_returns_false_for_empty_items() {
    let json = r#"{"items":[],"totalCount":0}"#;
    assert!(!parse_ready_items(json));
}

#[test]
fn parse_ready_items_returns_false_when_all_backlog() {
    let json = r#"{"items":[{"status":"Backlog","title":"A"},{"status":"Backlog","title":"B"}],"totalCount":2}"#;
    assert!(!parse_ready_items(json));
}

#[test]
fn parse_ready_items_returns_false_for_malformed_json() {
    let json = "not valid json at all";
    assert!(!parse_ready_items(json));
}

#[test]
fn parse_ready_items_returns_false_when_items_key_missing() {
    let json = r#"{"totalCount":0}"#;
    assert!(!parse_ready_items(json));
}

#[test]
fn parse_ready_items_returns_false_when_status_key_missing() {
    let json = r#"{"items":[{"title":"No status field"}],"totalCount":1}"#;
    assert!(!parse_ready_items(json));
}

// ── run_phase: CheckReady variant ───────────────────────────────────

#[test]
fn run_phase_check_ready_returns_some_phase() {
    // CheckReady calls check_ready_column which spawns `gh`, which will
    // fail in a test environment (no auth / network). The spawn failure
    // causes check_ready_column to return false, so run_phase should
    // return Some(GenerateTickets) (the no-items path).
    let config = Config {
        project: 1,
        owner: "test-owner".to_string(),
        max_cycles: 0,
        batch_size: 5,
    };
    let result = run_phase(&Phase::CheckReady, &config, &HashMap::new());
    match result {
        Some(phase) => assert_eq!(phase, Phase::GenerateTickets),
        None => panic!("expected Some, got None"),
    }
}

// ── load_direnv_env ─────────────────────────────────────────────────

#[test]
fn load_direnv_env_returns_hashmap() {
    // In test environments direnv may or may not be installed, and there
    // may or may not be a .envrc. Either way the function must return a
    // HashMap without panicking.
    let env = load_direnv_env();
    // We can only assert the type is correct (HashMap) and it didn't panic.
    // If direnv is not installed, the map will be empty.
    let _ = env.len();
}

// ── spawn_and_capture: extra_env propagation ────────────────────────

#[test]
fn spawn_and_capture_propagates_extra_env() {
    let mut env = HashMap::new();
    env.insert(
        "FLYWHEEL_TEST_VAR".to_string(),
        "hello_from_direnv".to_string(),
    );
    let result = spawn_and_capture("test", "sh", &["-c", "echo $FLYWHEEL_TEST_VAR"], &env);
    match result {
        Some(output) => assert!(
            output.contains("hello_from_direnv"),
            "expected env var in output, got: {output}"
        ),
        None => panic!("expected Some, got None"),
    }
}

#[test]
fn spawn_and_capture_empty_extra_env_works() {
    let result = spawn_and_capture("test", "echo", &["ok"], &HashMap::new());
    match result {
        Some(output) => assert!(output.contains("ok")),
        None => panic!("expected Some, got None"),
    }
}

// ── count_backlog_items ──────────────────────────────────────────

#[test]
fn count_backlog_items_mixed_statuses_counts_only_backlog() {
    let json = r#"{"items":[
        {"status":"Backlog","title":"A"},
        {"status":"Ready","title":"B"},
        {"status":"Done","title":"C"},
        {"status":"Backlog","title":"D"},
        {"status":"Ready","title":"E"}
    ],"totalCount":5}"#;
    assert_eq!(count_backlog_items(json), 2);
}

#[test]
fn count_backlog_items_empty_items_returns_zero() {
    let json = r#"{"items":[],"totalCount":0}"#;
    assert_eq!(count_backlog_items(json), 0);
}

#[test]
fn count_backlog_items_malformed_json_returns_zero() {
    let json = "not valid json at all";
    assert_eq!(count_backlog_items(json), 0);
}

#[test]
fn count_backlog_items_exactly_five_backlog_items() {
    let json = r#"{"items":[
        {"status":"Backlog","title":"A"},
        {"status":"Backlog","title":"B"},
        {"status":"Backlog","title":"C"},
        {"status":"Backlog","title":"D"},
        {"status":"Backlog","title":"E"}
    ],"totalCount":5}"#;
    assert_eq!(count_backlog_items(json), 5);
}

#[test]
fn count_backlog_items_four_backlog_items_below_threshold() {
    let json = r#"{"items":[
        {"status":"Backlog","title":"A"},
        {"status":"Backlog","title":"B"},
        {"status":"Backlog","title":"C"},
        {"status":"Backlog","title":"D"},
        {"status":"Ready","title":"E"}
    ],"totalCount":5}"#;
    assert_eq!(count_backlog_items(json), 4);
}
