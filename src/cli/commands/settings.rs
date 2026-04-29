use std::path::Path;

use anyhow::Result;

use crate::db::{CapturePolicy, CaptureSettings};

use crate::cli::display::format_duration_compact;
use crate::cli::formats::OutputFormat;
use crate::cli::output::{SettingsIgnoreListOutput, SettingsView};
use crate::cli::presentation::{emit_settings_ignore_list_output, emit_settings_view_output};
use crate::cli::schema::{
    SettingsApiKeyFilterArgs, SettingsArgs, SettingsCommand, SettingsIgnoreArgs,
    SettingsIgnoreCommand, SettingsIgnoreListArgs, SettingsOcrArgs, SettingsPauseArgs,
    SettingsResetArgs, SettingsRetentionArgs, SettingsShowArgs,
};

use super::mutation_support::require_text_or_json;
use super::runtime::open_or_init_db;

pub(in crate::cli) fn settings(db_path: &Path, args: &SettingsArgs) -> Result<()> {
    match &args.command {
        SettingsCommand::Show(args) => settings_show(db_path, args),
        SettingsCommand::Pause(args) => settings_pause(db_path, args),
        SettingsCommand::ApiKeyFilter(args) => settings_api_key_filter(db_path, args),
        SettingsCommand::Ocr(args) => settings_ocr(db_path, args),
        SettingsCommand::Retention(args) => settings_retention(db_path, args),
        SettingsCommand::Reset(args) => settings_reset(db_path, args),
        SettingsCommand::Ignore(args) => settings_ignore(db_path, args),
    }
}

fn settings_show(db_path: &Path, args: &SettingsShowArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "settings show")?;
    let db = open_or_init_db(db_path)?;
    let view = settings_view(db.capture_policy()?);
    emit_settings_view_output(format, &view)
}

fn settings_pause(db_path: &Path, args: &SettingsPauseArgs) -> Result<()> {
    let mut db = open_or_init_db(db_path)?;
    let settings = db.set_paused(args.state.is_on())?;
    let view = settings_view(CapturePolicy::new(settings, db.list_ignored_bundle_ids()?));
    emit_settings_view_output(OutputFormat::Text, &view)
}

fn settings_api_key_filter(db_path: &Path, args: &SettingsApiKeyFilterArgs) -> Result<()> {
    let mut db = open_or_init_db(db_path)?;
    let settings = db.set_api_key_filter_enabled(args.state.is_on())?;
    let view = settings_view(CapturePolicy::new(settings, db.list_ignored_bundle_ids()?));
    emit_settings_view_output(OutputFormat::Text, &view)
}

fn settings_ocr(db_path: &Path, args: &SettingsOcrArgs) -> Result<()> {
    let mut db = open_or_init_db(db_path)?;
    let settings = db.set_ocr_enabled(args.state.is_on())?;
    let view = settings_view(CapturePolicy::new(settings, db.list_ignored_bundle_ids()?));
    emit_settings_view_output(OutputFormat::Text, &view)
}

fn settings_retention(db_path: &Path, args: &SettingsRetentionArgs) -> Result<()> {
    let mut db = open_or_init_db(db_path)?;
    let settings = db.set_retention_seconds(args.value.retention_seconds())?;
    let view = settings_view(CapturePolicy::new(settings, db.list_ignored_bundle_ids()?));
    emit_settings_view_output(OutputFormat::Text, &view)
}

fn settings_reset(db_path: &Path, args: &SettingsResetArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "settings reset")?;
    let mut db = open_or_init_db(db_path)?;
    let view = settings_view(db.reset_capture_policy()?);
    emit_settings_view_output(format, &view)
}

fn settings_ignore(db_path: &Path, args: &SettingsIgnoreArgs) -> Result<()> {
    match &args.command {
        SettingsIgnoreCommand::Add(args) => settings_ignore_add(db_path, &args.bundle_id),
        SettingsIgnoreCommand::Remove(args) => settings_ignore_remove(db_path, &args.bundle_id),
        SettingsIgnoreCommand::List(args) => settings_ignore_list(db_path, args),
    }
}

fn settings_ignore_add(db_path: &Path, bundle_id: &str) -> Result<()> {
    let mut db = open_or_init_db(db_path)?;
    db.add_ignored_bundle_id(bundle_id)?;
    let output = SettingsIgnoreListOutput {
        ignored_bundle_ids: db.list_ignored_bundle_ids()?,
    };
    emit_settings_ignore_list_output(OutputFormat::Text, &output)
}

fn settings_ignore_remove(db_path: &Path, bundle_id: &str) -> Result<()> {
    let mut db = open_or_init_db(db_path)?;
    db.remove_ignored_bundle_id(bundle_id)?;
    let output = SettingsIgnoreListOutput {
        ignored_bundle_ids: db.list_ignored_bundle_ids()?,
    };
    emit_settings_ignore_list_output(OutputFormat::Text, &output)
}

fn settings_ignore_list(db_path: &Path, args: &SettingsIgnoreListArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "settings ignore list")?;
    let db = open_or_init_db(db_path)?;
    let output = SettingsIgnoreListOutput {
        ignored_bundle_ids: db.list_ignored_bundle_ids()?,
    };
    emit_settings_ignore_list_output(format, &output)
}

fn settings_view(policy: CapturePolicy) -> SettingsView {
    SettingsView {
        paused: policy.settings().paused(),
        api_key_filter_enabled: policy.settings().api_key_filter_enabled(),
        ocr_enabled: policy.settings().ocr_enabled(),
        retention_seconds: policy.settings().retention_seconds(),
        retention: render_retention_value(policy.settings()),
        ignored_bundle_ids: policy.ignored_bundle_ids().to_vec(),
    }
}

fn render_retention_value(settings: &CaptureSettings) -> String {
    settings
        .retention_seconds()
        .map(format_duration_compact)
        .unwrap_or_else(|| "forever".to_string())
}
