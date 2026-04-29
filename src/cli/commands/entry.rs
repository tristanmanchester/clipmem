use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use super::super::human::render_service_status_human;
use super::super::presentation::emit_json_or_text;
use super::super::schema::{
    AgentsArgs, AgentsCommand, AgentsContextArgs, Command, HermesArgs, HermesCommand, OpenClawArgs,
    OpenClawCommand, ServiceArgs, ServiceCommand, ServiceProvidersArgs, ServiceStatusArgs,
    SetupArgs,
};
use super::super::service as service_api;
use super::super::service::ServiceProviderStatus;
use super::super::service::{
    render_service_action_text, render_service_status_text, render_setup_text,
};
use super::agents::{
    agent_context, hermes_doctor, hermes_install_skill, hermes_uninstall_skill, openclaw_doctor,
    openclaw_install_skill, openclaw_uninstall_skill, packaged_hermes_skill,
    packaged_openclaw_skill,
};
use super::app::app;
use super::archive_mutate::{
    export_snapshot_bytes, forget_snapshot, purge_snapshots, restore_snapshot,
};
use super::doctor::doctor;
use super::mutation_support::require_text_or_json;
use super::ocr::ocr;
use super::retrieval::{recall, recent, search, show_snapshot, stats, timeline};
use super::runtime::{capture_once, watch};
use super::settings::settings;
use super::storage::storage;

pub(in crate::cli) fn run_command(command: Command, db_path: &Path) -> Result<()> {
    match command {
        Command::Agents(args) => agents(db_path, &args),
        Command::Setup(args) => setup(db_path, &args),
        Command::Service(args) => service(db_path, &args),
        Command::App(args) => app(db_path, &args),
        Command::Watch(args) => watch(db_path, &args),
        Command::CaptureOnce(args) => capture_once(db_path, &args),
        Command::Search(args) => search(db_path, &args),
        Command::Recent(args) => recent(db_path, &args),
        Command::Timeline(args) => timeline(db_path, &args),
        Command::Stats(args) => stats(db_path, &args),
        Command::Recall(args) => recall(db_path, &args),
        Command::Get(args) => show_snapshot(db_path, &args),
        Command::Export(args) => export_snapshot_bytes(db_path, &args),
        Command::Restore(args) => restore_snapshot(db_path, &args),
        Command::Forget(args) => forget_snapshot(db_path, &args),
        Command::Purge(args) => purge_snapshots(db_path, &args),
        Command::Storage(args) => storage(db_path, &args),
        Command::Ocr(args) => ocr(db_path, &args),
        Command::Settings(args) => settings(db_path, &args),
        Command::Doctor(args) => doctor(db_path, &args),
    }
}

pub(in crate::cli) fn setup(db_path: &Path, _args: &SetupArgs) -> Result<()> {
    let report = service_api::setup(db_path)?;
    print!("{}", render_setup_text(&report));
    Ok(())
}

pub(in crate::cli) fn service(db_path: &Path, args: &ServiceArgs) -> Result<()> {
    match &args.command {
        ServiceCommand::Providers(args) => service_providers(db_path, args),
        ServiceCommand::Start => {
            let report = service_api::start(db_path)?;
            print!("{}", render_service_action_text(&report));
            Ok(())
        }
        ServiceCommand::Stop => {
            let report = service_api::stop(db_path)?;
            print!("{}", render_service_action_text(&report));
            Ok(())
        }
        ServiceCommand::Status(status_args) => service_status(db_path, status_args),
        ServiceCommand::Uninstall => {
            let report = service_api::uninstall(db_path)?;
            print!("{}", render_service_action_text(&report));
            Ok(())
        }
    }
}

#[derive(Debug, Serialize)]
struct ServiceProvidersOutput {
    preferred_provider: String,
    preferred_provider_reason: String,
    providers: Vec<ServiceProviderStatus>,
    conflict: bool,
}

pub(in crate::cli) fn service_providers(db_path: &Path, args: &ServiceProvidersArgs) -> Result<()> {
    let format = require_text_or_json(args.output.resolved()?, "service providers")?;
    let report = service_api::status_report(db_path)?;
    let output = ServiceProvidersOutput {
        preferred_provider: report.preferred_provider.clone(),
        preferred_provider_reason: report.preferred_provider_reason.clone(),
        providers: vec![report.homebrew.clone(), report.launchagent.clone()],
        conflict: report.conflict,
    };
    emit_json_or_text(
        matches!(format, super::super::formats::OutputFormat::Json),
        &output,
        render_service_providers_text,
    )
}

fn render_service_providers_text(output: &ServiceProvidersOutput) -> String {
    let mut rendered = format!(
        "preferred_provider={} conflict={}\n",
        output.preferred_provider, output.conflict
    );
    for provider in &output.providers {
        rendered.push_str(&format!(
            "{} state={:?} installed={} loaded={} running={}\n",
            provider.provider.as_str(),
            provider.state,
            provider.installed,
            provider.loaded,
            provider.running
        ));
    }
    rendered
}

pub(in crate::cli) fn service_status(db_path: &Path, args: &ServiceStatusArgs) -> Result<()> {
    let report = service_api::status_report(db_path)?;
    if args.human {
        print!("{}", render_service_status_human(&report));
    } else if args.json {
        emit_json_or_text(true, &report, render_service_status_text)?;
    } else {
        print!("{}", render_service_status_text(&report));
    }
    Ok(())
}

pub(in crate::cli) fn agents(db_path: &Path, args: &AgentsArgs) -> Result<()> {
    match &args.command {
        AgentsCommand::Context(args) => agents_context(db_path, args),
        AgentsCommand::Openclaw(args) => openclaw(args),
        AgentsCommand::Hermes(args) => hermes(args),
    }
}

pub(in crate::cli) fn agents_context(db_path: &Path, args: &AgentsContextArgs) -> Result<()> {
    let format = args.output.resolved()?;
    agent_context(db_path, format)
}

pub(in crate::cli) fn openclaw(args: &OpenClawArgs) -> Result<()> {
    match &args.command {
        OpenClawCommand::InstallSkill(args) => openclaw_install_skill(args),
        OpenClawCommand::UninstallSkill(args) => openclaw_uninstall_skill(args),
        OpenClawCommand::PrintSkill => {
            print!("{}", packaged_openclaw_skill());
            Ok(())
        }
        OpenClawCommand::Doctor(args) => openclaw_doctor(args),
    }
}

pub(in crate::cli) fn hermes(args: &HermesArgs) -> Result<()> {
    match &args.command {
        HermesCommand::InstallSkill(args) => hermes_install_skill(args),
        HermesCommand::UninstallSkill(args) => hermes_uninstall_skill(args),
        HermesCommand::PrintSkill => {
            print!("{}", packaged_hermes_skill());
            Ok(())
        }
        HermesCommand::Doctor(args) => hermes_doctor(args),
    }
}
