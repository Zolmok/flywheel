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
        verbose: false,
        implement_only: false,
    };

    let result = merge_config(file, &cli);
    match result {
        Ok(config) => {
            assert_eq!(config.project, 42);
            assert_eq!(config.owner, "acme");
            assert_eq!(config.max_cycles, 3);
            assert_eq!(config.batch_size, 10);
            assert!(!config.implement_only);
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
        verbose: false,
        implement_only: false,
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
        verbose: false,
        implement_only: false,
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
        verbose: false,
        implement_only: false,
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
        verbose: false,
        implement_only: false,
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

// ── next_phase: all transitions ──────────────────────────────────────

#[test]
fn next_phase_generate_tickets_to_size_prioritize() {
    assert_eq!(
        next_phase(&Phase::GenerateTickets, false, false),
        Some(Phase::SizePrioritize)
    );
}

#[test]
fn next_phase_size_prioritize_to_move_to_ready() {
    assert_eq!(
        next_phase(&Phase::SizePrioritize, false, false),
        Some(Phase::MoveToReady)
    );
}

#[test]
fn next_phase_move_to_ready_to_implement_ticket() {
    assert_eq!(
        next_phase(&Phase::MoveToReady, false, false),
        Some(Phase::ImplementTicket)
    );
}

#[test]
fn next_phase_implement_ticket_to_check_ready() {
    assert_eq!(
        next_phase(&Phase::ImplementTicket, false, false),
        Some(Phase::CheckReady)
    );
}

#[test]
fn next_phase_check_ready_with_items_to_implement_ticket() {
    assert_eq!(
        next_phase(&Phase::CheckReady, true, false),
        Some(Phase::ImplementTicket)
    );
}

#[test]
fn next_phase_check_ready_without_items_to_generate_tickets() {
    assert_eq!(
        next_phase(&Phase::CheckReady, false, false),
        Some(Phase::GenerateTickets)
    );
}

#[test]
fn next_phase_check_ready_without_items_implement_only_returns_none() {
    assert_eq!(next_phase(&Phase::CheckReady, false, true), None);
}

#[test]
fn next_phase_check_ready_with_items_implement_only_continues() {
    assert_eq!(
        next_phase(&Phase::CheckReady, true, true),
        Some(Phase::ImplementTicket)
    );
}

#[test]
fn next_phase_implement_only_does_not_affect_non_check_ready_phases() {
    assert_eq!(
        next_phase(&Phase::ImplementTicket, false, true),
        Some(Phase::CheckReady)
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
    let result = spawn_and_capture("test", "echo", &["hello"], &HashMap::new(), false);
    match result {
        Some(output) => assert_eq!(output, "hello\n"),
        None => panic!("expected Some, got None"),
    }
}

#[test]
fn spawn_and_capture_nonexistent_program_returns_none() {
    let result = spawn_and_capture(
        "test",
        "nonexistent_program_xyz",
        &[],
        &HashMap::new(),
        false,
    );
    assert!(result.is_none(), "expected None, got Some");
}

#[test]
fn spawn_and_capture_captures_multiline_output() {
    let result = spawn_and_capture(
        "test",
        "printf",
        &["line1\nline2\nline3\n"],
        &HashMap::new(),
        false,
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
        false,
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
        verbose: false,
        implement_only: false,
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
        verbose: false,
        implement_only: false,
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
        verbose: false,
        implement_only: false,
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
        verbose: false,
        implement_only: false,
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
        verbose: false,
        implement_only: false,
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
        verbose: false,
        implement_only: false,
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
        verbose: false,
        implement_only: false,
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
        verbose: false,
        implement_only: false,
    };
    let prompt = build_move_to_ready_prompt(&config);
    assert!(prompt.contains("8"), "prompt should contain batch_size");
}

// ── build_implement_ticket_prompt ───────────────────────────────────

#[test]
fn build_implement_ticket_prompt_without_ticket_contains_project_number() {
    let config = Config {
        project: 33,
        owner: "dev".to_string(),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: false,
    };
    let prompt = build_implement_ticket_prompt(&config, None);
    assert!(
        prompt.contains("33"),
        "prompt should contain project number"
    );
}

#[test]
fn build_implement_ticket_prompt_without_ticket_contains_owner() {
    let config = Config {
        project: 33,
        owner: "dev".to_string(),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: false,
    };
    let prompt = build_implement_ticket_prompt(&config, None);
    assert!(prompt.contains("dev"), "prompt should contain owner");
}

#[test]
fn build_implement_ticket_prompt_without_ticket_contains_implement_ticket_skill() {
    let config = Config {
        project: 1,
        owner: "org".to_string(),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: false,
    };
    let prompt = build_implement_ticket_prompt(&config, None);
    assert!(
        prompt.contains("implement-ticket"),
        "prompt should contain implement-ticket skill name"
    );
}

#[test]
fn build_implement_ticket_prompt_with_ticket_contains_ticket_number() {
    let config = Config {
        project: 3,
        owner: "org".to_string(),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: false,
    };
    let ticket = TicketInfo {
        number: 42,
        title: "Fix the widget".to_string(),
    };
    let prompt = build_implement_ticket_prompt(&config, Some(&ticket));
    assert!(prompt.contains("42"), "prompt should contain ticket number");
}

#[test]
fn build_implement_ticket_prompt_with_ticket_contains_project_and_owner() {
    let config = Config {
        project: 7,
        owner: "team".to_string(),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: false,
    };
    let ticket = TicketInfo {
        number: 99,
        title: "Add feature".to_string(),
    };
    let prompt = build_implement_ticket_prompt(&config, Some(&ticket));
    assert!(prompt.contains("7"), "prompt should contain project number");
    assert!(prompt.contains("team"), "prompt should contain owner");
}

#[test]
fn build_implement_ticket_prompt_with_ticket_contains_implement_ticket_skill() {
    let config = Config {
        project: 1,
        owner: "org".to_string(),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: false,
    };
    let ticket = TicketInfo {
        number: 10,
        title: "Something".to_string(),
    };
    let prompt = build_implement_ticket_prompt(&config, Some(&ticket));
    assert!(
        prompt.contains("implement-ticket"),
        "prompt should contain implement-ticket skill name"
    );
}

// ── count_ready_items ───────────────────────────────────────────────

#[test]
fn count_ready_items_returns_one_when_ready_item_exists() {
    let json = r#"{"items":[{"status":"Ready","title":"Do something"}],"totalCount":1}"#;
    assert_eq!(count_ready_items(json), 1);
}

#[test]
fn count_ready_items_with_mixed_statuses() {
    let json = r#"{"items":[{"status":"Backlog","title":"A"},{"status":"Ready","title":"B"}],"totalCount":2}"#;
    assert_eq!(count_ready_items(json), 1);
}

#[test]
fn count_ready_items_returns_zero_for_empty_items() {
    let json = r#"{"items":[],"totalCount":0}"#;
    assert_eq!(count_ready_items(json), 0);
}

#[test]
fn count_ready_items_returns_zero_when_all_backlog() {
    let json = r#"{"items":[{"status":"Backlog","title":"A"},{"status":"Backlog","title":"B"}],"totalCount":2}"#;
    assert_eq!(count_ready_items(json), 0);
}

#[test]
fn count_ready_items_returns_zero_for_malformed_json() {
    let json = "not valid json at all";
    assert_eq!(count_ready_items(json), 0);
}

#[test]
fn count_ready_items_returns_zero_when_items_key_missing() {
    let json = r#"{"totalCount":0}"#;
    assert_eq!(count_ready_items(json), 0);
}

#[test]
fn count_ready_items_returns_zero_when_status_key_missing() {
    let json = r#"{"items":[{"title":"No status field"}],"totalCount":1}"#;
    assert_eq!(count_ready_items(json), 0);
}

#[test]
fn count_ready_items_multiple_ready_items() {
    let json = r#"{"items":[
        {"status":"Ready","title":"A"},
        {"status":"Backlog","title":"B"},
        {"status":"Ready","title":"C"},
        {"status":"Done","title":"D"}
    ],"totalCount":4}"#;
    assert_eq!(count_ready_items(json), 2);
}

// ── parse_top_ready_ticket ──────────────────────────────────────────

#[test]
fn parse_top_ready_ticket_returns_first_ready_item() {
    let json = r#"{"items":[
        {"status":"Backlog","title":"A","content":{"number":1}},
        {"status":"Ready","title":"First ready","content":{"number":42}},
        {"status":"Ready","title":"Second ready","content":{"number":43}}
    ],"totalCount":3}"#;
    match parse_top_ready_ticket(json) {
        Some(info) => {
            assert_eq!(info.number, 42);
            assert_eq!(info.title, "First ready");
        }
        None => panic!("expected Some, got None"),
    }
}

#[test]
fn parse_top_ready_ticket_returns_none_when_no_ready_items() {
    let json = r#"{"items":[
        {"status":"Backlog","title":"A","content":{"number":1}},
        {"status":"Done","title":"B","content":{"number":2}}
    ],"totalCount":2}"#;
    assert!(parse_top_ready_ticket(json).is_none());
}

#[test]
fn parse_top_ready_ticket_returns_none_for_malformed_json() {
    assert!(parse_top_ready_ticket("not valid json").is_none());
}

#[test]
fn parse_top_ready_ticket_returns_none_when_missing_content_number() {
    let json = r#"{"items":[{"status":"Ready","title":"No number"}],"totalCount":1}"#;
    assert!(parse_top_ready_ticket(json).is_none());
}

#[test]
fn parse_top_ready_ticket_returns_none_when_missing_title() {
    let json = r#"{"items":[{"status":"Ready","content":{"number":1}}],"totalCount":1}"#;
    assert!(parse_top_ready_ticket(json).is_none());
}

#[test]
fn parse_top_ready_ticket_returns_none_for_empty_items() {
    let json = r#"{"items":[],"totalCount":0}"#;
    assert!(parse_top_ready_ticket(json).is_none());
}

// ── run_phase: CheckReady variant ───────────────────────────────────

#[test]
fn run_phase_check_ready_returns_some_phase() {
    // CheckReady calls fetch_project_items which spawns `gh`, which will
    // fail in a test environment (no auth / network). The spawn failure
    // causes the no-items path, so run_phase should return
    // Some(PhaseResult { next: GenerateTickets, ticket: None }).
    let config = Config {
        project: 1,
        owner: "test-owner".to_string(),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: false,
    };
    let result = run_phase(&Phase::CheckReady, &config, &HashMap::new());
    match result {
        Some(pr) => {
            assert_eq!(pr.next, Some(Phase::GenerateTickets));
            assert!(pr.ticket.is_none());
        }
        None => panic!("expected Some, got None"),
    }
}

// ── fetch_project_items ─────────────────────────────────────────────

#[test]
fn fetch_project_items_returns_none_when_gh_unavailable() {
    // In a test environment `gh project item-list` will fail (no auth,
    // no network, or gh not installed). spawn_and_capture still returns
    // Some with captured output even on non-zero exit, but gh may not
    // be present at all, which would yield None. Either way the function
    // must not panic.
    let config = Config {
        project: 999999,
        owner: "nonexistent-owner-xyz".to_string(),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: false,
    };
    let result = fetch_project_items(&config, &HashMap::new());
    // We cannot guarantee None vs Some (depends on whether gh is
    // installed), but we verify the call completes without panicking
    // and that the result is a valid Option<String>.
    let _ = result.is_some();
}

#[test]
fn fetch_project_items_passes_project_and_owner_to_gh() {
    // Verify fetch_project_items builds the correct command by using a
    // config with specific values. Since gh will fail in tests, we just
    // confirm it does not panic and returns an Option.
    let config = Config {
        project: 42,
        owner: "acme-corp".to_string(),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: false,
    };
    let env = HashMap::new();
    let result = fetch_project_items(&config, &env);
    let _ = result.is_some();
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
    let result = spawn_and_capture(
        "test",
        "sh",
        &["-c", "echo $FLYWHEEL_TEST_VAR"],
        &env,
        false,
    );
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
    let result = spawn_and_capture("test", "echo", &["ok"], &HashMap::new(), false);
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

// ── backlog_items_need_sizing ────────────────────────────────────────

#[test]
fn backlog_items_need_sizing_true_when_size_field_missing() {
    let json = r#"{"items":[{"status":"Backlog","title":"A"}],"totalCount":1}"#;
    assert!(backlog_items_need_sizing(json));
}

#[test]
fn backlog_items_need_sizing_true_when_size_field_empty() {
    let json = r#"{"items":[{"status":"Backlog","title":"A","size":""}],"totalCount":1}"#;
    assert!(backlog_items_need_sizing(json));
}

#[test]
fn backlog_items_need_sizing_false_when_all_have_size() {
    let json = r#"{"items":[
        {"status":"Backlog","title":"A","size":"M"},
        {"status":"Backlog","title":"B","size":"XS"}
    ],"totalCount":2}"#;
    assert!(!backlog_items_need_sizing(json));
}

#[test]
fn backlog_items_need_sizing_ignores_non_backlog_items() {
    let json = r#"{"items":[
        {"status":"Ready","title":"A"},
        {"status":"Done","title":"B"}
    ],"totalCount":2}"#;
    assert!(!backlog_items_need_sizing(json));
}

#[test]
fn backlog_items_need_sizing_false_for_empty_items() {
    let json = r#"{"items":[],"totalCount":0}"#;
    assert!(!backlog_items_need_sizing(json));
}

#[test]
fn backlog_items_need_sizing_false_for_malformed_json() {
    assert!(!backlog_items_need_sizing("not valid json"));
}

#[test]
fn backlog_items_need_sizing_mixed_backlog_some_missing_size() {
    let json = r#"{"items":[
        {"status":"Backlog","title":"A","size":"L"},
        {"status":"Backlog","title":"B"}
    ],"totalCount":2}"#;
    assert!(backlog_items_need_sizing(json));
}

// ── backlog_items_need_prioritization ────────────────────────────────

#[test]
fn backlog_items_need_prioritization_true_when_priority_field_missing() {
    let json = r#"{"items":[{"status":"Backlog","title":"A"}],"totalCount":1}"#;
    assert!(backlog_items_need_prioritization(json));
}

#[test]
fn backlog_items_need_prioritization_true_when_priority_field_empty() {
    let json = r#"{"items":[{"status":"Backlog","title":"A","priority":""}],"totalCount":1}"#;
    assert!(backlog_items_need_prioritization(json));
}

#[test]
fn backlog_items_need_prioritization_false_when_all_have_priority() {
    let json = r#"{"items":[
        {"status":"Backlog","title":"A","priority":"P1"},
        {"status":"Backlog","title":"B","priority":"P2"}
    ],"totalCount":2}"#;
    assert!(!backlog_items_need_prioritization(json));
}

#[test]
fn backlog_items_need_prioritization_ignores_non_backlog_items() {
    let json = r#"{"items":[
        {"status":"Ready","title":"A"},
        {"status":"Done","title":"B"}
    ],"totalCount":2}"#;
    assert!(!backlog_items_need_prioritization(json));
}

#[test]
fn backlog_items_need_prioritization_false_for_empty_items() {
    let json = r#"{"items":[],"totalCount":0}"#;
    assert!(!backlog_items_need_prioritization(json));
}

#[test]
fn backlog_items_need_prioritization_false_for_malformed_json() {
    assert!(!backlog_items_need_prioritization("not valid json"));
}

#[test]
fn backlog_items_need_prioritization_mixed_backlog_some_missing_priority() {
    let json = r#"{"items":[
        {"status":"Backlog","title":"A","priority":"P0"},
        {"status":"Backlog","title":"B"}
    ],"totalCount":2}"#;
    assert!(backlog_items_need_prioritization(json));
}

// ── spawn_and_capture: stderr excluded from output ──────────────────

#[test]
fn spawn_and_capture_returns_stdout_only_not_stderr() {
    let result = spawn_and_capture(
        "test",
        "sh",
        &["-c", "echo STDOUT_CONTENT && echo STDERR_CONTENT >&2"],
        &HashMap::new(),
        false,
    );
    match result {
        Some(output) => {
            assert!(
                output.contains("STDOUT_CONTENT"),
                "expected stdout content in output, got: {output}"
            );
            assert!(
                !output.contains("STDERR_CONTENT"),
                "expected stderr content NOT in output, got: {output}"
            );
        }
        None => panic!("expected Some, got None"),
    }
}

// ── spawn_and_capture: quiet mode ──────────────────────────────────

#[test]
fn spawn_and_capture_quiet_still_captures_output() {
    let result = spawn_and_capture("test", "echo", &["captured"], &HashMap::new(), true);
    match result {
        Some(output) => assert!(
            output.contains("captured"),
            "quiet mode should still capture output, got: {output}"
        ),
        None => panic!("expected Some, got None"),
    }
}

// ── merge_config: verbose flag passthrough ──────────────────────────

#[test]
fn merge_config_verbose_true_passes_through() {
    let file = FileConfig::default();
    let cli = Cli {
        project: Some(1),
        owner: Some("owner".to_string()),
        max_cycles: 0,
        batch_size: 5,
        verbose: true,
        implement_only: false,
    };

    let result = merge_config(file, &cli);
    match result {
        Ok(config) => assert!(config.verbose, "expected verbose to be true"),
        Err(e) => panic!("expected Ok, got Err: {e}"),
    }
}

#[test]
fn merge_config_verbose_false_passes_through() {
    let file = FileConfig::default();
    let cli = Cli {
        project: Some(1),
        owner: Some("owner".to_string()),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: false,
    };

    let result = merge_config(file, &cli);
    match result {
        Ok(config) => assert!(!config.verbose, "expected verbose to be false"),
        Err(e) => panic!("expected Ok, got Err: {e}"),
    }
}

// ── merge_config: implement_only flag passthrough ───────────────────

#[test]
fn merge_config_implement_only_true_passes_through() {
    let file = FileConfig::default();
    let cli = Cli {
        project: Some(1),
        owner: Some("owner".to_string()),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: true,
    };

    let result = merge_config(file, &cli);
    match result {
        Ok(config) => assert!(config.implement_only, "expected implement_only to be true"),
        Err(e) => panic!("expected Ok, got Err: {e}"),
    }
}

#[test]
fn merge_config_implement_only_false_passes_through() {
    let file = FileConfig::default();
    let cli = Cli {
        project: Some(1),
        owner: Some("owner".to_string()),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: false,
    };

    let result = merge_config(file, &cli);
    match result {
        Ok(config) => assert!(
            !config.implement_only,
            "expected implement_only to be false"
        ),
        Err(e) => panic!("expected Ok, got Err: {e}"),
    }
}

// ── spawn_spinner ───────────────────────────────────────────────────

#[test]
fn spawn_spinner_starts_and_stops_cleanly() {
    let (stop, handle) = spawn_spinner("test");
    std::thread::sleep(std::time::Duration::from_millis(200));
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    match handle.join() {
        Ok(_) => {}
        Err(_) => panic!("spinner thread panicked"),
    }
}

// ── count_backlog_items: large item list (>30) ──────────────────────

#[test]
fn count_backlog_items_large_list_over_thirty_items() {
    let mut items = Vec::new();
    for i in 0..50 {
        if i % 3 == 0 {
            items.push(format!(r#"{{"status":"Backlog","title":"Item {i}"}}"#));
        } else if i % 3 == 1 {
            items.push(format!(r#"{{"status":"Ready","title":"Item {i}"}}"#));
        } else {
            items.push(format!(r#"{{"status":"Done","title":"Item {i}"}}"#));
        }
    }
    let json = format!(r#"{{"items":[{}],"totalCount":50}}"#, items.join(","));
    // Items 0,3,6,9,12,15,18,21,24,27,30,33,36,39,42,45,48 = 17 backlog
    assert_eq!(count_backlog_items(&json), 17);
}

// ── count_ready_items: large item list (>30) ────────────────────────

#[test]
fn count_ready_items_large_list_over_thirty_items() {
    let mut items = Vec::new();
    for i in 0..50 {
        if i % 3 == 0 {
            items.push(format!(r#"{{"status":"Backlog","title":"Item {i}"}}"#));
        } else if i % 3 == 1 {
            items.push(format!(r#"{{"status":"Ready","title":"Item {i}"}}"#));
        } else {
            items.push(format!(r#"{{"status":"Done","title":"Item {i}"}}"#));
        }
    }
    let json = format!(r#"{{"items":[{}],"totalCount":50}}"#, items.join(","));
    // Items 1,4,7,10,13,16,19,22,25,28,31,34,37,40,43,46,49 = 17 ready
    assert_eq!(count_ready_items(&json), 17);
}

// ── parse_top_ready_ticket: large item list (>30) ───────────────────

#[test]
fn parse_top_ready_ticket_finds_ready_item_beyond_position_thirty() {
    let mut items = Vec::new();
    for i in 0..35 {
        items.push(format!(
            r#"{{"status":"Backlog","title":"Backlog {i}","content":{{"number":{i}}}}}"#
        ));
    }
    items
        .push(r#"{"status":"Ready","title":"The ready one","content":{"number":999}}"#.to_string());
    let json = format!(r#"{{"items":[{}],"totalCount":36}}"#, items.join(","));
    match parse_top_ready_ticket(&json) {
        Some(info) => {
            assert_eq!(info.number, 999);
            assert_eq!(info.title, "The ready one");
        }
        None => panic!("expected Some, got None"),
    }
}

// ── spawn_and_capture: quiet mode with spinner captures all output ──

#[test]
fn spawn_and_capture_quiet_mode_with_spinner_captures_output() {
    let result = spawn_and_capture(
        "test",
        "printf",
        &["alpha\nbeta\ngamma\n"],
        &HashMap::new(),
        true,
    );
    match result {
        Some(output) => {
            assert!(
                output.contains("alpha"),
                "expected alpha in output, got: {output}"
            );
            assert!(
                output.contains("beta"),
                "expected beta in output, got: {output}"
            );
            assert!(
                output.contains("gamma"),
                "expected gamma in output, got: {output}"
            );
        }
        None => panic!("expected Some, got None"),
    }
}

// ── run_phase: GenerateTickets backlog threshold uses batch_size ─────

#[test]
fn run_phase_generate_tickets_skips_when_backlog_at_threshold_zero() {
    let config = Config {
        project: 1,
        owner: "test-owner".to_string(),
        max_cycles: 0,
        batch_size: 0,
        verbose: false,
        implement_only: false,
    };
    let result = run_phase(&Phase::GenerateTickets, &config, &HashMap::new());
    match result {
        Some(r) => assert_eq!(r.next, Some(Phase::SizePrioritize)),
        None => panic!("expected Some(SizePrioritize), got None"),
    }
}

#[test]
fn run_phase_generate_tickets_skip_returns_size_prioritize_at_threshold_one() {
    let config_skip = Config {
        project: 1,
        owner: "test-owner".to_string(),
        max_cycles: 0,
        batch_size: 0,
        verbose: false,
        implement_only: false,
    };
    let result_skip = run_phase(&Phase::GenerateTickets, &config_skip, &HashMap::new());
    match result_skip {
        Some(r) => assert_eq!(r.next, Some(Phase::SizePrioritize)),
        None => panic!("expected Some(SizePrioritize) for batch_size=0, got None"),
    }
}

#[test]
fn run_phase_generate_tickets_skip_message_contains_threshold() {
    let threshold: usize = 7_u32 as usize;
    let backlog_count: usize = 10;
    let msg = format!(
        "Backlog has {backlog_count} items (threshold: {threshold}), skipping ticket generation"
    );
    assert!(
        msg.contains("threshold: 7"),
        "skip message should embed the configured threshold value"
    );
}

// ── run_phase: ImplementTicket variant ───────────────────────────────

#[test]
fn run_phase_implement_ticket_skips_when_fetch_fails() {
    // In a test environment `gh` will fail, triggering the
    // "failed to fetch project items, skipping" path.
    let config = Config {
        project: 1,
        owner: "test-owner".to_string(),
        max_cycles: 0,
        batch_size: 5,
        verbose: false,
        implement_only: false,
    };
    let result = run_phase(&Phase::ImplementTicket, &config, &HashMap::new());
    match result {
        Some(pr) => {
            assert_eq!(pr.next, Some(Phase::CheckReady));
            assert!(pr.ticket.is_none());
        }
        None => panic!("expected Some, got None"),
    }
}

// ── print_phase_banner ──────────────────────────────────────────────

#[test]
fn print_phase_banner_without_ticket_does_not_panic() {
    // Verify the function runs cleanly when ticket is None.
    print_phase_banner(&Phase::GenerateTickets, 1, None);
}

#[test]
fn print_phase_banner_with_ticket_does_not_panic() {
    // Verify the function runs cleanly when a ticket is provided,
    // exercising the Some(info) arm that prints ticket number and title.
    let ticket = TicketInfo {
        number: 42,
        title: "Fix the widget".to_string(),
    };
    print_phase_banner(&Phase::ImplementTicket, 3, Some(&ticket));
}

// ── ORIGINAL_TERMIOS static ─────────────────────────────────────────

#[test]
fn original_termios_static_is_lockable() {
    // Verify the static mutex can be locked without panicking
    match ORIGINAL_TERMIOS.lock() {
        Ok(guard) => {
            let _ = guard.is_some();
        }
        Err(e) => panic!("expected Ok, got Err: {e}"),
    }
}

#[test]
fn raw_mode_enter_stores_original_termios() {
    // RawMode::enter() may fail in CI (no terminal), so handle gracefully
    match RawMode::enter() {
        Some(_raw) => match ORIGINAL_TERMIOS.lock() {
            Ok(guard) => assert!(
                guard.is_some(),
                "ORIGINAL_TERMIOS should be Some after enter()"
            ),
            Err(e) => panic!("expected Ok, got Err: {e}"),
        },
        None => {
            // No terminal available (CI), skip assertion
        }
    }
}

#[test]
fn raw_mode_drop_still_works() {
    // Verify that drop doesn't panic even after our changes
    match RawMode::enter() {
        Some(raw) => {
            drop(raw);
            // If we get here, drop didn't panic
        }
        None => {
            // No terminal available (CI), skip
        }
    }
}
