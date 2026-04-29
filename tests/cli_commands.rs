#[path = "cli_commands/support.rs"]
mod support;

pub(crate) use self::support::*;

#[path = "cli_commands/app_commands.rs"]
mod app_commands;
#[path = "cli_commands/formats_and_settings.rs"]
mod formats_and_settings;
#[path = "cli_commands/help_and_stats.rs"]
mod help_and_stats;
#[path = "cli_commands/hermes_agents.rs"]
mod hermes_agents;
#[path = "cli_commands/pagination_and_exports.rs"]
mod pagination_and_exports;
#[path = "cli_commands/recall_and_human.rs"]
mod recall_and_human;
#[path = "cli_commands/search_and_retrieval.rs"]
mod search_and_retrieval;
#[path = "cli_commands/service_setup.rs"]
mod service_setup;
#[path = "cli_commands/storage_and_openclaw.rs"]
mod storage_and_openclaw;
