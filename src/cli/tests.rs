use std::path::PathBuf;

use clap::Parser;

use crate::db::{RetrievalKind, SearchMode, TimelineSort};

use super::errors::classify_command_error;
use super::formats::{OutputFormat, ProgressFormat, RecallOutputFormat};
use super::parsing::RetentionValue;
use super::schema::{
    AgentsCommand, AppCommand, AppLaunchAtLoginCommand, AppPreferenceKey, AppSettingsCommand,
    AppUpdateCheckCommand, Cli, Command, HermesCommand, OcrCommand, OpenClawCommand,
    ServiceCommand, SettingsCommand, SettingsIgnoreCommand, StorageCommand,
};
use super::CliExitCode;

#[test]
fn watch_command_parses_global_db_and_runtime_flags() {
    let cli = Cli::parse_from([
        "clipmem",
        "--db",
        "/tmp/clipmem.sqlite3",
        "watch",
        "--interval-ms",
        "250",
        "--quiet",
        "--skip-initial",
    ]);

    assert_eq!(cli.db, Some(PathBuf::from("/tmp/clipmem.sqlite3")));
    match cli.command {
        Command::Watch(args) => {
            assert_eq!(args.interval_ms, 250);
            assert!(args.quiet);
            assert!(args.skip_initial);
        }
        other => panic!("expected watch command, got {other:?}"),
    }
}

#[test]
fn service_providers_parses_json_output() {
    let cli = Cli::parse_from(["clipmem", "service", "providers", "--format", "json"]);

    match cli.command {
        Command::Service(args) => match args.command {
            ServiceCommand::Providers(args) => {
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected service providers command, got {other:?}"),
        },
        other => panic!("expected service command, got {other:?}"),
    }
}

#[test]
fn service_revision_parses_json_output() {
    let cli = Cli::parse_from(["clipmem", "service", "revision", "--format", "json"]);

    match cli.command {
        Command::Service(args) => match args.command {
            ServiceCommand::Revision(args) => {
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected service revision command, got {other:?}"),
        },
        other => panic!("expected service command, got {other:?}"),
    }
}

#[test]
fn agents_openclaw_commands_parse_install_and_doctor_flags() {
    let install_cli = Cli::parse_from([
        "clipmem",
        "agents",
        "openclaw",
        "install-skill",
        "--shared",
        "--dest",
        "/tmp/clipboard-memory",
        "--force",
    ]);

    match install_cli.command {
        Command::Agents(args) => match args.command {
            AgentsCommand::Openclaw(args) => match args.command {
                OpenClawCommand::InstallSkill(args) => {
                    assert!(args.shared);
                    assert_eq!(args.dest, Some(PathBuf::from("/tmp/clipboard-memory")));
                    assert!(args.force);
                }
                other => panic!("expected install-skill command, got {other:?}"),
            },
            other => panic!("expected openclaw command, got {other:?}"),
        },
        other => panic!("expected agents command, got {other:?}"),
    }

    let doctor_cli = Cli::parse_from([
        "clipmem",
        "agents",
        "openclaw",
        "doctor",
        "--dest",
        "/tmp/clipboard-memory",
    ]);
    match doctor_cli.command {
        Command::Agents(args) => match args.command {
            AgentsCommand::Openclaw(args) => match args.command {
                OpenClawCommand::Doctor(args) => {
                    assert_eq!(args.dest, Some(PathBuf::from("/tmp/clipboard-memory")));
                    assert!(!args.shared);
                }
                other => panic!("expected doctor command, got {other:?}"),
            },
            other => panic!("expected openclaw command, got {other:?}"),
        },
        other => panic!("expected agents command, got {other:?}"),
    }
}

#[test]
fn agents_context_parses_json_output() {
    let cli = Cli::parse_from(["clipmem", "agents", "context", "--format", "json"]);

    match cli.command {
        Command::Agents(args) => match args.command {
            AgentsCommand::Context(args) => {
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected context command, got {other:?}"),
        },
        other => panic!("expected agents command, got {other:?}"),
    }
}

#[test]
fn app_settings_commands_parse_key_and_output() {
    let cli = Cli::parse_from([
        "clipmem",
        "app",
        "settings",
        "set",
        "default-query-mode",
        "timeline",
        "--format",
        "json",
    ]);

    match cli.command {
        Command::App(args) => match args.command {
            AppCommand::Settings(args) => match args.command {
                AppSettingsCommand::Set(args) => {
                    assert_eq!(args.key, AppPreferenceKey::DefaultQueryMode);
                    assert_eq!(args.value, "timeline");
                    assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
                }
                other => panic!("expected app settings set command, got {other:?}"),
            },
            other => panic!("expected app settings command, got {other:?}"),
        },
        other => panic!("expected app command, got {other:?}"),
    }
}

#[test]
fn app_state_commands_parse_output() {
    let launch_cli = Cli::parse_from([
        "clipmem",
        "app",
        "launch-at-login",
        "set",
        "on",
        "--format",
        "json",
    ]);
    let update_cli = Cli::parse_from(["clipmem", "app", "update-check", "run", "--format", "json"]);
    let quit_cli = Cli::parse_from(["clipmem", "app", "quit", "--format", "json"]);

    match launch_cli.command {
        Command::App(args) => match args.command {
            AppCommand::LaunchAtLogin(args) => match args.command {
                AppLaunchAtLoginCommand::Set(args) => {
                    assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
                }
                other => panic!("expected app launch-at-login set command, got {other:?}"),
            },
            other => panic!("expected launch-at-login command, got {other:?}"),
        },
        other => panic!("expected app command, got {other:?}"),
    }

    match update_cli.command {
        Command::App(args) => match args.command {
            AppCommand::UpdateCheck(args) => match args.command {
                AppUpdateCheckCommand::Run(args) => {
                    assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
                }
                other => panic!("expected app update-check run command, got {other:?}"),
            },
            other => panic!("expected update-check command, got {other:?}"),
        },
        other => panic!("expected app command, got {other:?}"),
    }

    match quit_cli.command {
        Command::App(args) => match args.command {
            AppCommand::Quit(args) => {
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected app quit command, got {other:?}"),
        },
        other => panic!("expected app command, got {other:?}"),
    }
}

#[test]
fn agents_hermes_commands_parse_install_doctor_print_and_uninstall() {
    let install_cli = Cli::parse_from([
        "clipmem",
        "agents",
        "hermes",
        "install-skill",
        "--dest",
        "/tmp/clipboard-memory",
        "--force",
    ]);

    match install_cli.command {
        Command::Agents(args) => match args.command {
            AgentsCommand::Hermes(args) => match args.command {
                HermesCommand::InstallSkill(args) => {
                    assert_eq!(args.dest, Some(PathBuf::from("/tmp/clipboard-memory")));
                    assert!(args.force);
                }
                other => panic!("expected install-skill command, got {other:?}"),
            },
            other => panic!("expected hermes command, got {other:?}"),
        },
        other => panic!("expected agents command, got {other:?}"),
    }

    let doctor_cli = Cli::parse_from([
        "clipmem",
        "agents",
        "hermes",
        "doctor",
        "--dest",
        "/tmp/clipboard-memory",
    ]);
    match doctor_cli.command {
        Command::Agents(args) => match args.command {
            AgentsCommand::Hermes(args) => match args.command {
                HermesCommand::Doctor(args) => {
                    assert_eq!(args.dest, Some(PathBuf::from("/tmp/clipboard-memory")));
                }
                other => panic!("expected doctor command, got {other:?}"),
            },
            other => panic!("expected hermes command, got {other:?}"),
        },
        other => panic!("expected agents command, got {other:?}"),
    }

    let print_cli = Cli::parse_from(["clipmem", "agents", "hermes", "print-skill"]);
    match print_cli.command {
        Command::Agents(args) => match args.command {
            AgentsCommand::Hermes(args) => {
                assert!(matches!(args.command, HermesCommand::PrintSkill));
            }
            other => panic!("expected hermes command, got {other:?}"),
        },
        other => panic!("expected agents command, got {other:?}"),
    }

    let uninstall_cli = Cli::parse_from(["clipmem", "agents", "hermes", "uninstall-skill"]);
    match uninstall_cli.command {
        Command::Agents(args) => match args.command {
            AgentsCommand::Hermes(args) => {
                assert!(matches!(args.command, HermesCommand::UninstallSkill(_)));
            }
            other => panic!("expected hermes command, got {other:?}"),
        },
        other => panic!("expected agents command, got {other:?}"),
    }
}

#[test]
fn search_command_parses_explicit_mode() {
    let cli = Cli::parse_from(["clipmem", "search", "--mode", "literal", "50%"]);

    match cli.command {
        Command::Search(args) => {
            assert!(matches!(args.mode, SearchMode::Literal));
            assert_eq!(args.query, "50%");
            assert_eq!(args.output.resolved().unwrap(), OutputFormat::Text);
        }
        other => panic!("expected search command, got {other:?}"),
    }
}

#[test]
fn list_commands_parse_output_format_and_cursor() {
    let cli = Cli::parse_from([
        "clipmem", "recent", "--limit", "5", "--cursor", "abcd", "--format", "jsonl",
    ]);

    match cli.command {
        Command::Recent(args) => {
            assert_eq!(args.cursor.as_deref(), Some("abcd"));
            assert_eq!(args.output.resolved().unwrap(), OutputFormat::Jsonl);
        }
        other => panic!("expected recent command, got {other:?}"),
    }
}

#[test]
fn timeline_command_parses_filters_and_sort() {
    let cli = Cli::parse_from([
        "clipmem",
        "timeline",
        "--since",
        "2026-04-16T09:00:00Z",
        "--until",
        "2026-04-16T10:00:00Z",
        "--hours",
        "24",
        "--limit",
        "5",
        "--cursor",
        "abcd",
        "--sort",
        "asc",
        "--format",
        "md",
    ]);

    match cli.command {
        Command::Timeline(args) => {
            assert_eq!(args.filters.since.as_deref(), Some("2026-04-16T09:00:00Z"));
            assert_eq!(args.filters.until.as_deref(), Some("2026-04-16T10:00:00Z"));
            assert_eq!(args.filters.hours, Some(24));
            assert_eq!(args.limit, 5);
            assert_eq!(args.cursor.as_deref(), Some("abcd"));
            assert_eq!(args.sort, TimelineSort::Asc);
            assert_eq!(args.output.resolved().unwrap(), OutputFormat::Md);
        }
        other => panic!("expected timeline command, got {other:?}"),
    }
}

#[test]
fn timeline_command_defaults_to_desc_sort() {
    let cli = Cli::parse_from(["clipmem", "timeline"]);

    match cli.command {
        Command::Timeline(args) => {
            assert_eq!(args.sort, TimelineSort::Desc);
        }
        other => panic!("expected timeline command, got {other:?}"),
    }
}

#[test]
fn recall_command_parses_optional_query_and_flags() {
    let cli = Cli::parse_from([
        "clipmem",
        "recall",
        "git status",
        "--format",
        "json",
        "--limit",
        "4",
        "--hours",
        "24",
        "--full",
        "--quote",
        "--min-score",
        "0.7",
        "--prefer-recent",
        "--prefer-app",
        "terminal",
    ]);

    match cli.command {
        Command::Recall(args) => {
            assert_eq!(args.query.as_deref(), Some("git status"));
            assert_eq!(args.output.resolved().unwrap(), RecallOutputFormat::Json);
            assert_eq!(args.limit, 4);
            assert_eq!(args.filters.hours, Some(24));
            assert!(args.full);
            assert!(args.quote);
            assert_eq!(args.min_score, Some(0.7));
            assert!(args.prefer_recent);
            assert_eq!(args.prefer_app.as_deref(), Some("terminal"));
        }
        other => panic!("expected recall command, got {other:?}"),
    }
}

#[test]
fn recall_command_defaults_to_markdown_output() {
    let cli = Cli::parse_from(["clipmem", "recall"]);

    match cli.command {
        Command::Recall(args) => {
            assert_eq!(args.output.resolved().unwrap(), RecallOutputFormat::Md);
        }
        other => panic!("expected recall command, got {other:?}"),
    }
}

#[test]
fn json_alias_resolves_to_json_output() {
    let cli = Cli::parse_from(["clipmem", "get", "42", "--json"]);

    match cli.command {
        Command::Get(args) => {
            assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
        }
        other => panic!("expected get command, got {other:?}"),
    }
}

#[test]
fn json_alias_rejects_non_json_format() {
    let error = super::run_from(["clipmem", "search", "git", "--json", "--format", "md"])
        .expect_err("invalid output alias combination should fail");

    assert!(error
        .to_string()
        .contains("`--json` is only compatible with `--format json`"));
}

#[test]
fn settings_mutation_output_alias_conflicts_fail_during_validation() {
    let cli = Cli::parse_from([
        "clipmem", "settings", "pause", "on", "--json", "--format", "md",
    ]);

    let error = super::validate::validate_cli(&cli)
        .expect_err("settings mutation output conflicts should fail before execution");

    assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    assert!(error
        .to_string()
        .contains("`--json` is only compatible with `--format json`"));
}

#[test]
fn timeline_command_rejects_inverted_time_range() {
    let error = super::run_from([
        "clipmem",
        "timeline",
        "--since",
        "2026-04-16T11:00:00Z",
        "--until",
        "2026-04-16T10:00:00Z",
    ])
    .expect_err("invalid time range should fail");

    assert!(error
        .to_string()
        .contains("`--since` must be earlier than or equal to `--until`"));
}

#[test]
fn get_command_parses_shared_filters() {
    let cli = Cli::parse_from([
        "clipmem",
        "get",
        "42",
        "--events",
        "3",
        "--app",
        "terminal",
        "--bundle-id",
        "com.apple.Terminal",
        "--kind",
        "url",
        "--has-url",
        "--min-bytes",
        "20",
        "--max-bytes",
        "200",
        "--format",
        "json",
    ]);

    match cli.command {
        Command::Get(args) => {
            assert_eq!(args.snapshot_id, 42);
            assert_eq!(args.events, 3);
            assert_eq!(args.filters.app.as_deref(), Some("terminal"));
            assert_eq!(
                args.filters.bundle_id.as_deref(),
                Some("com.apple.Terminal")
            );
            assert!(matches!(args.filters.kind, Some(RetrievalKind::Url)));
            assert!(args.filters.has_url);
            assert_eq!(args.filters.min_bytes, Some(20));
            assert_eq!(args.filters.max_bytes, Some(200));
            assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
        }
        other => panic!("expected get command, got {other:?}"),
    }
}

#[test]
fn export_command_parses_shared_filters() {
    let cli = Cli::parse_from([
        "clipmem",
        "export",
        "42",
        "--item",
        "1",
        "--uti",
        "public.url",
        "--out",
        "/tmp/clipmem.url",
        "--since",
        "2026-04-16T09:00:00Z",
        "--hours",
        "24",
        "--app",
        "safari",
        "--kind",
        "file",
        "--has-file-url",
    ]);

    match cli.command {
        Command::Export(args) => {
            assert_eq!(args.snapshot_id, 42);
            assert_eq!(args.item, 1);
            assert_eq!(args.uti, "public.url");
            assert_eq!(args.out, PathBuf::from("/tmp/clipmem.url"));
            assert!(!args.force);
            assert_eq!(args.filters.since.as_deref(), Some("2026-04-16T09:00:00Z"));
            assert_eq!(args.filters.hours, Some(24));
            assert_eq!(args.filters.app.as_deref(), Some("safari"));
            assert!(matches!(args.filters.kind, Some(RetrievalKind::File)));
            assert!(args.filters.has_file_url);
        }
        other => panic!("expected export command, got {other:?}"),
    }
}

#[test]
fn shared_filters_reject_inverted_byte_window() {
    let error = super::run_from([
        "clipmem",
        "search",
        "git",
        "--min-bytes",
        "200",
        "--max-bytes",
        "20",
    ])
    .expect_err("invalid byte range should fail");

    assert!(error
        .to_string()
        .contains("`--min-bytes` must be less than or equal to `--max-bytes`"));
}

#[test]
fn shared_filters_reject_byte_values_above_sqlite_range() {
    let error = Cli::try_parse_from([
        "clipmem",
        "search",
        "git",
        "--min-bytes",
        "9223372036854775808",
    ])
    .expect_err("byte filters above the SQLite range should fail during argument parsing");

    assert!(error.to_string().contains("exceeds supported range"));
}

#[test]
fn shared_filters_reject_zero_hours() {
    let error = super::run_from(["clipmem", "recent", "--hours", "0"])
        .expect_err("zero-hour filter should fail");

    assert!(error
        .to_string()
        .contains("`--hours` must be greater than zero"));
}

#[test]
fn shared_filters_reject_empty_app_and_bundle_id_values() {
    let app_error = super::run_from(["clipmem", "recent", "--app", "   "])
        .expect_err("empty app filter should fail");
    assert!(app_error.to_string().contains("--app cannot be empty"));

    let bundle_error = super::run_from(["clipmem", "timeline", "--bundle-id", ""])
        .expect_err("empty bundle id filter should fail");
    assert!(bundle_error
        .to_string()
        .contains("--bundle-id cannot be empty"));
}

#[test]
fn shared_filters_normalize_hours_away_when_since_is_present() {
    let cli = Cli::parse_from([
        "clipmem",
        "search",
        "--since",
        "2026-04-16T09:00:00Z",
        "--hours",
        "24",
        "git",
    ]);

    match cli.command {
        Command::Search(args) => {
            let filters = args.filters.normalized().expect("filters should normalize");
            assert_eq!(filters.since(), Some("2026-04-16T09:00:00Z"));
            assert_eq!(filters.hours(), None);
        }
        other => panic!("expected search command, got {other:?}"),
    }
}

#[test]
fn search_command_rejects_zero_limit() {
    let result = Cli::try_parse_from(["clipmem", "search", "--limit", "0", "git"]);

    assert!(result.is_err());
}

#[test]
fn export_command_parses_required_arguments() {
    let cli = Cli::parse_from([
        "clipmem",
        "export",
        "42",
        "--item",
        "1",
        "--uti",
        "public.png",
        "--out",
        "/tmp/clipmem.bin",
        "--force",
    ]);

    match cli.command {
        Command::Export(args) => {
            assert_eq!(args.snapshot_id, 42);
            assert_eq!(args.item, 1);
            assert_eq!(args.uti, "public.png");
            assert_eq!(args.out, PathBuf::from("/tmp/clipmem.bin"));
            assert!(args.force);
        }
        other => panic!("expected export command, got {other:?}"),
    }
}

#[test]
fn restore_forget_and_purge_commands_parse_expected_arguments() {
    let restore_cli = Cli::parse_from(["clipmem", "restore", "42"]);
    match restore_cli.command {
        Command::Restore(args) => assert_eq!(args.snapshot_id, 42),
        other => panic!("expected restore command, got {other:?}"),
    }

    let forget_cli = Cli::parse_from(["clipmem", "forget", "42"]);
    match forget_cli.command {
        Command::Forget(args) => assert_eq!(args.snapshot_id, 42),
        other => panic!("expected forget command, got {other:?}"),
    }

    let purge_cli = Cli::parse_from(["clipmem", "purge", "--older-than", "30d", "--dry-run"]);
    match purge_cli.command {
        Command::Purge(args) => {
            assert_eq!(args.older_than.raw(), "30d");
            assert_eq!(args.older_than.seconds(), 30 * 24 * 60 * 60);
            assert!(args.dry_run);
        }
        other => panic!("expected purge command, got {other:?}"),
    }
}

#[test]
fn storage_commands_parse_expected_arguments() {
    let compact_cli = Cli::parse_from([
        "clipmem",
        "storage",
        "compact",
        "--dry-run",
        "--format",
        "json",
    ]);
    match compact_cli.command {
        Command::Storage(args) => match args.command {
            StorageCommand::Compact(args) => {
                assert!(args.dry_run);
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected storage compact command, got {other:?}"),
        },
        other => panic!("expected storage command, got {other:?}"),
    }

    let candidates_cli = Cli::parse_from([
        "clipmem",
        "storage",
        "image-candidates",
        "--limit",
        "5",
        "--format",
        "json",
    ]);
    match candidates_cli.command {
        Command::Storage(args) => match args.command {
            StorageCommand::ImageCandidates(args) => {
                assert_eq!(args.limit, 5);
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected storage image-candidates command, got {other:?}"),
        },
        other => panic!("expected storage command, got {other:?}"),
    }

    let optimize_cli = Cli::parse_from([
        "clipmem",
        "storage",
        "optimize-images",
        "--no-compact",
        "--limit",
        "50",
        "--format",
        "json",
    ]);
    match optimize_cli.command {
        Command::Storage(args) => match args.command {
            StorageCommand::OptimizeImages(args) => {
                assert!(!args.dry_run);
                assert!(args.no_compact);
                assert_eq!(args.limit, 50);
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected storage optimize-images command, got {other:?}"),
        },
        other => panic!("expected storage command, got {other:?}"),
    }

    let progress_cli = Cli::parse_from([
        "clipmem",
        "storage",
        "optimize-images",
        "--progress",
        "jsonl",
    ]);
    match progress_cli.command {
        Command::Storage(args) => match args.command {
            StorageCommand::OptimizeImages(args) => {
                assert_eq!(args.progress, Some(ProgressFormat::Jsonl));
            }
            other => panic!("expected storage optimize-images command, got {other:?}"),
        },
        other => panic!("expected storage command, got {other:?}"),
    }
}

#[test]
fn storage_optimize_images_progress_rejects_output_format_flags() {
    let error = super::run_from([
        "clipmem",
        "storage",
        "optimize-images",
        "--progress",
        "jsonl",
        "--format",
        "json",
    ])
    .expect_err("progress output should reject explicit output format");

    assert!(error
        .to_string()
        .contains("`--progress jsonl` cannot be combined"));
}

#[test]
fn settings_commands_parse_policy_variants() {
    let show_cli = Cli::parse_from(["clipmem", "settings", "show", "--format", "json"]);
    match show_cli.command {
        Command::Settings(args) => match args.command {
            SettingsCommand::Show(args) => {
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected settings show command, got {other:?}"),
        },
        other => panic!("expected settings command, got {other:?}"),
    }

    let pause_cli = Cli::parse_from(["clipmem", "settings", "pause", "on"]);
    match pause_cli.command {
        Command::Settings(args) => match args.command {
            SettingsCommand::Pause(args) => assert!(args.state.is_on()),
            other => panic!("expected settings pause command, got {other:?}"),
        },
        other => panic!("expected settings command, got {other:?}"),
    }

    let filter_cli = Cli::parse_from(["clipmem", "settings", "api-key-filter", "on"]);
    match filter_cli.command {
        Command::Settings(args) => match args.command {
            SettingsCommand::ApiKeyFilter(args) => assert!(args.state.is_on()),
            other => panic!("expected settings api-key-filter command, got {other:?}"),
        },
        other => panic!("expected settings command, got {other:?}"),
    }

    let ocr_cli = Cli::parse_from(["clipmem", "settings", "ocr", "on"]);
    match ocr_cli.command {
        Command::Settings(args) => match args.command {
            SettingsCommand::Ocr(args) => assert!(args.state.is_on()),
            other => panic!("expected settings ocr command, got {other:?}"),
        },
        other => panic!("expected settings command, got {other:?}"),
    }

    let retention_cli = Cli::parse_from(["clipmem", "settings", "retention", "forever"]);
    match retention_cli.command {
        Command::Settings(args) => match args.command {
            SettingsCommand::Retention(args) => {
                assert!(matches!(args.value, RetentionValue::Forever));
                assert_eq!(args.value.retention_seconds(), None);
            }
            other => panic!("expected settings retention command, got {other:?}"),
        },
        other => panic!("expected settings command, got {other:?}"),
    }

    let reset_cli = Cli::parse_from(["clipmem", "settings", "reset", "--format", "json"]);
    match reset_cli.command {
        Command::Settings(args) => match args.command {
            SettingsCommand::Reset(args) => {
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected settings reset command, got {other:?}"),
        },
        other => panic!("expected settings command, got {other:?}"),
    }

    let ignore_cli =
        Cli::parse_from(["clipmem", "settings", "ignore", "add", "com.apple.Terminal"]);
    match ignore_cli.command {
        Command::Settings(args) => match args.command {
            SettingsCommand::Ignore(args) => match args.command {
                SettingsIgnoreCommand::Add(args) => {
                    assert_eq!(args.bundle_id, "com.apple.Terminal");
                }
                other => panic!("expected settings ignore add command, got {other:?}"),
            },
            other => panic!("expected settings ignore command, got {other:?}"),
        },
        other => panic!("expected settings command, got {other:?}"),
    }
}

#[test]
fn ocr_commands_parse_status_and_run_options() {
    let status_cli = Cli::parse_from(["clipmem", "ocr", "status", "--format", "json"]);
    match status_cli.command {
        Command::Ocr(args) => match args.command {
            OcrCommand::Status(args) => {
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected ocr status command, got {other:?}"),
        },
        other => panic!("expected ocr command, got {other:?}"),
    }

    let candidates_cli = Cli::parse_from([
        "clipmem",
        "ocr",
        "candidates",
        "--limit",
        "5",
        "--snapshot",
        "42",
        "--format",
        "json",
    ]);
    match candidates_cli.command {
        Command::Ocr(args) => match args.command {
            OcrCommand::Candidates(args) => {
                assert_eq!(args.limit, 5);
                assert_eq!(args.snapshot, Some(42));
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected ocr candidates command, got {other:?}"),
        },
        other => panic!("expected ocr command, got {other:?}"),
    }

    let raw_hash = "a".repeat(64);
    let get_cli = Cli::parse_from([
        "clipmem",
        "ocr",
        "get",
        raw_hash.as_str(),
        "--format",
        "json",
    ]);
    match get_cli.command {
        Command::Ocr(args) => match args.command {
            OcrCommand::Get(args) => {
                assert_eq!(args.raw_sha256, raw_hash);
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected ocr get command, got {other:?}"),
        },
        other => panic!("expected ocr command, got {other:?}"),
    }

    let raw_hash = "b".repeat(64);
    let clear_cli = Cli::parse_from([
        "clipmem",
        "ocr",
        "clear",
        raw_hash.as_str(),
        "--format",
        "json",
    ]);
    match clear_cli.command {
        Command::Ocr(args) => match args.command {
            OcrCommand::Clear(args) => {
                assert_eq!(args.raw_sha256, raw_hash);
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected ocr clear command, got {other:?}"),
        },
        other => panic!("expected ocr command, got {other:?}"),
    }

    let run_cli = Cli::parse_from([
        "clipmem",
        "ocr",
        "run",
        "--limit",
        "7",
        "--snapshot",
        "42",
        "--retry-failed",
        "--format",
        "json",
    ]);
    match run_cli.command {
        Command::Ocr(args) => match args.command {
            OcrCommand::Run(args) => {
                assert_eq!(args.limit, 7);
                assert_eq!(args.snapshot, Some(42));
                assert!(args.retry_failed);
                assert_eq!(args.output.resolved().unwrap(), OutputFormat::Json);
            }
            other => panic!("expected ocr run command, got {other:?}"),
        },
        other => panic!("expected ocr command, got {other:?}"),
    }
}

#[test]
fn duration_parser_accepts_single_unit_values_and_rejects_invalid_ones() {
    let days_cli = Cli::parse_from(["clipmem", "purge", "--older-than", "30d"]);
    match days_cli.command {
        Command::Purge(args) => assert_eq!(args.older_than.seconds(), 30 * 24 * 60 * 60),
        other => panic!("expected purge command, got {other:?}"),
    }

    let hours_cli = Cli::parse_from(["clipmem", "purge", "--older-than", "12h"]);
    match hours_cli.command {
        Command::Purge(args) => assert_eq!(args.older_than.seconds(), 12 * 60 * 60),
        other => panic!("expected purge command, got {other:?}"),
    }

    let minutes_cli = Cli::parse_from(["clipmem", "purge", "--older-than", "15m"]);
    match minutes_cli.command {
        Command::Purge(args) => assert_eq!(args.older_than.seconds(), 15 * 60),
        other => panic!("expected purge command, got {other:?}"),
    }

    let compound = super::run_from(["clipmem", "purge", "--older-than", "1h30m"])
        .expect_err("compound durations should fail");
    assert!(compound.to_string().contains("expected an integer amount"));

    let zero = super::run_from(["clipmem", "purge", "--older-than", "0d"])
        .expect_err("zero durations should fail");
    assert!(zero.to_string().contains("greater than zero"));

    let out_of_range = super::run_from(["clipmem", "purge", "--older-than", "106751991167301d"])
        .expect_err("durations above the SQLite range should fail during argument parsing");
    assert!(out_of_range.to_string().contains("exceeds supported range"));
}

#[test]
fn get_command_rejects_zero_event_limit() {
    let result = Cli::try_parse_from(["clipmem", "get", "42", "--events", "0"]);

    assert!(result.is_err());
}

#[test]
fn command_error_classifier_marks_platform_failures() {
    let error = classify_command_error(anyhow::anyhow!(
        "clipboard capture is only supported on macOS"
    ));

    assert_eq!(error.exit_code(), CliExitCode::PlatformError);
    assert!(error.to_string().contains("only supported on macOS"));
}
