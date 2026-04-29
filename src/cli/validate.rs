use clap::error::ErrorKind;

use super::errors::{clap_error, clap_error_from_value, CliValueError};
use super::schema::{
    AppCommand, AppLaunchAtLoginCommand, AppSettingsCommand, AppUpdateCheckCommand, Cli, Command,
    OcrCommand, ServiceCommand, SettingsCommand, SettingsIgnoreCommand, StorageCommand,
    StorageOptimizeImagesArgs,
};

fn validate_value<T>(
    result: std::result::Result<T, CliValueError>,
) -> std::result::Result<T, clap::Error> {
    result.map_err(clap_error_from_value)
}

pub(super) fn validate_cli(cli: &Cli) -> std::result::Result<(), clap::Error> {
    match &cli.command {
        Command::Agents(_args) => {}
        Command::Setup(_) => {}
        Command::Service(args) => match &args.command {
            ServiceCommand::Providers(args) => {
                validate_value(args.output.resolved())?;
            }
            ServiceCommand::Revision(args) => {
                validate_value(args.output.resolved())?;
            }
            ServiceCommand::Status(args) => {
                validate_json_human_flags(args.json, args.human)?;
            }
            ServiceCommand::Start | ServiceCommand::Stop | ServiceCommand::Uninstall => {}
        },
        Command::App(args) => match &args.command {
            AppCommand::Settings(args) => match &args.command {
                AppSettingsCommand::Show(args) => {
                    validate_value(args.output.resolved())?;
                }
                AppSettingsCommand::Set(args) => {
                    validate_value(args.output.resolved())?;
                }
                AppSettingsCommand::Clear(args) => {
                    validate_value(args.output.resolved())?;
                }
            },
            AppCommand::LaunchAtLogin(args) => match &args.command {
                AppLaunchAtLoginCommand::Show(args) => {
                    validate_value(args.output.resolved())?;
                }
                AppLaunchAtLoginCommand::Set(args) => {
                    validate_value(args.output.resolved())?;
                }
                AppLaunchAtLoginCommand::Clear(args) => {
                    validate_value(args.output.resolved())?;
                }
            },
            AppCommand::UpdateCheck(args) => match &args.command {
                AppUpdateCheckCommand::Show(args) => {
                    validate_value(args.output.resolved())?;
                }
                AppUpdateCheckCommand::Run(args) => {
                    validate_value(args.output.resolved())?;
                }
                AppUpdateCheckCommand::Clear(args) => {
                    validate_value(args.output.resolved())?;
                }
            },
            AppCommand::Quit(args) => {
                validate_value(args.output.resolved())?;
            }
        },
        Command::Search(args) => {
            validate_value(args.output.resolved())?;
            validate_value(args.filters.normalized())?;
        }
        Command::Recent(args) => {
            validate_value(args.output.resolved())?;
            validate_value(args.filters.normalized())?;
        }
        Command::Timeline(args) => {
            validate_value(args.output.resolved())?;
            validate_value(args.filters.normalized())?;
        }
        Command::Stats(args) => {
            validate_value(args.output.resolved())?;
            validate_value(args.filters.normalized())?;
        }
        Command::Ocr(args) => match &args.command {
            OcrCommand::Status(args) => {
                validate_value(args.output.resolved())?;
            }
            OcrCommand::Candidates(args) => {
                validate_value(args.output.resolved())?;
            }
            OcrCommand::Get(args) => {
                validate_value(args.output.resolved())?;
            }
            OcrCommand::Clear(args) => {
                validate_value(args.output.resolved())?;
            }
            OcrCommand::Run(args) => {
                validate_value(args.output.resolved())?;
            }
        },
        Command::Recall(args) => {
            validate_value(args.output.resolved())?;
            validate_value(args.filters.normalized())?;
        }
        Command::Get(args) => {
            validate_value(args.output.resolved())?;
            validate_value(args.filters.normalized())?;
        }
        Command::Export(args) => {
            validate_value(args.output.resolved())?;
            validate_value(args.filters.normalized())?;
        }
        Command::Restore(args) => {
            validate_value(args.output.resolved())?;
        }
        Command::Forget(args) => {
            validate_value(args.output.resolved())?;
        }
        Command::Purge(args) => {
            validate_value(args.output.resolved())?;
        }
        Command::Storage(args) => match &args.command {
            StorageCommand::Compact(args) => {
                validate_value(args.output.resolved())?;
            }
            StorageCommand::ImageCandidates(args) => {
                validate_value(args.output.resolved())?;
            }
            StorageCommand::OptimizeImages(args) => {
                validate_optimize_images_progress(args)?;
                validate_value(args.output.resolved())?;
            }
        },
        Command::Settings(args) => match &args.command {
            SettingsCommand::Show(args) => {
                validate_value(args.output.resolved())?;
            }
            SettingsCommand::Pause(_) | SettingsCommand::ApiKeyFilter(_) => {}
            SettingsCommand::Ocr(_) => {}
            SettingsCommand::Retention(_) => {}
            SettingsCommand::Reset(args) => {
                validate_value(args.output.resolved())?;
            }
            SettingsCommand::Ignore(args) => match &args.command {
                SettingsIgnoreCommand::Add(_) | SettingsIgnoreCommand::Remove(_) => {}
                SettingsIgnoreCommand::List(args) => {
                    validate_value(args.output.resolved())?;
                }
            },
        },
        Command::Watch(_) => {}
        Command::CaptureOnce(args) => {
            validate_json_human_flags(args.json, args.human)?;
        }
        Command::Doctor(args) => {
            validate_json_human_flags(args.json, args.human)?;
        }
    }

    Ok(())
}

fn validate_optimize_images_progress(
    args: &StorageOptimizeImagesArgs,
) -> std::result::Result<(), clap::Error> {
    if let Some(progress) = args.progress {
        if args.output.format.is_some() || args.output.json || args.output.human {
            return Err(clap_error(
                ErrorKind::ArgumentConflict,
                format!(
                    "`--progress {}` cannot be combined with `--format`, `--json`, or `--human`",
                    progress.as_str()
                ),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_json_human_flags(
    json: bool,
    human: bool,
) -> std::result::Result<(), clap::Error> {
    if json && human {
        Err(clap_error(
            ErrorKind::ArgumentConflict,
            "`--human` cannot be combined with `--json`",
        ))
    } else {
        Ok(())
    }
}
