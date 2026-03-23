use super::*;

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
