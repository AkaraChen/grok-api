mod cli;
mod error;
mod infra;
mod model;
mod service;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::{
    AuthCommand, Cli, ConfigCommand, Globals, ImageCommand, Resource, VideoCommand,
    VideoTaskCommand, print_root_help, source_from_globals,
};
use crate::error::Result;
use crate::infra::output::{emit, emit_text};
use crate::model::config::{DEFAULT_POLL_INTERVAL_SECS, OutputFormat};
use crate::model::media::{ImageGenerateRequest, ResponseFormat, VideoGenerateRequest};
use crate::service::auth::AuthContext;
use crate::service::{auth, config as config_service, image, video};

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 || args.iter().any(|arg| arg == "--help" || arg == "-h") && args.len() == 2 {
        print_root_help();
        return Ok(());
    }

    let cli = Cli::parse();
    let Some(resource) = cli.resource else {
        print_root_help();
        return Ok(());
    };
    dispatch(cli.globals, resource).await
}

async fn dispatch(globals: Globals, resource: Resource) -> Result<()> {
    let mut config = config_service::load()?;
    if let Some(base_url) = &globals.base_url {
        config.base_url = base_url.clone();
    }
    if let Some(timeout) = globals.timeout {
        config.timeout = timeout;
    }
    if let Some(output) = &globals.output {
        config.output = output.clone().into();
    }
    let output = config.output;
    let ctx = AuthContext {
        config,
        api_key_override: globals.api_key.clone(),
        source_override: source_from_globals(&globals),
    };

    match resource {
        Resource::Auth { command } => auth_cmd(&ctx, &globals, output, command).await,
        Resource::Image { command } => image_cmd(&ctx, &globals, output, command).await,
        Resource::Video { command } => video_cmd(&ctx, &globals, output, command).await,
        Resource::Config { command } => config_cmd(&globals, output, command),
    }
}

async fn auth_cmd(
    ctx: &AuthContext,
    globals: &Globals,
    output: OutputFormat,
    command: AuthCommand,
) -> Result<()> {
    match command {
        AuthCommand::Login {
            api_key,
            oauth,
            device_auth,
            from_grok_cli,
        } => {
            let status = if let Some(api_key) = api_key.or(globals.api_key.clone()) {
                if globals.dry_run {
                    return emit_text(output, globals.quiet, "dry-run: would save API key to ~/.grok-api");
                }
                auth::login_with_api_key(&api_key)?
            } else if from_grok_cli || globals.from_grok_cli {
                if globals.dry_run {
                    return emit_text(
                        output,
                        globals.quiet,
                        "dry-run: would read credentials from ~/.grok/auth.json",
                    );
                }
                auth::login_from_grok_cli()?
            } else if oauth || device_auth || !globals.non_interactive {
                if globals.dry_run {
                    return emit_text(
                        output,
                        globals.quiet,
                        "dry-run: would run `GROK_HOME=~/.grok-api grok login`",
                    );
                }
                auth::login_with_official_grok(device_auth)?
            } else {
                return Err(crate::error::Error::message(
                    "non-interactive login needs --api-key or --from-grok-cli",
                ));
            };
            emit(
                output,
                globals.quiet,
                format!(
                    "authenticated via {} ({})",
                    status.source,
                    status.email.as_deref().unwrap_or(status.home.as_str())
                ),
                &status,
            )
        }
        AuthCommand::Status => {
            let status = auth::status(ctx)?;
            emit(
                output,
                globals.quiet,
                format!(
                    "authenticated={} source={} home={}",
                    status.authenticated, status.source, status.home
                ),
                &status,
            )
        }
        AuthCommand::Refresh { device_auth } => {
            if globals.dry_run {
                return emit_text(output, globals.quiet, "dry-run: would refresh ~/.grok-api via grok login");
            }
            let status = auth::refresh(device_auth)?;
            emit(output, globals.quiet, "refreshed ~/.grok-api login", &status)
        }
        AuthCommand::Logout { yes } => {
            if globals.dry_run {
                return emit_text(output, globals.quiet, "dry-run: would delete ~/.grok-api/auth.json");
            }
            let deleted = auth::logout(yes || globals.non_interactive)?;
            emit(
                output,
                globals.quiet,
                if deleted {
                    "cleared ~/.grok-api/auth.json"
                } else {
                    "no ~/.grok-api/auth.json to clear"
                },
                &serde_json::json!({ "cleared": deleted }),
            )
        }
    }
}

async fn image_cmd(
    ctx: &AuthContext,
    globals: &Globals,
    output: OutputFormat,
    command: ImageCommand,
) -> Result<()> {
    let ImageCommand::Generate {
        prompt,
        aspect_ratio,
        n,
        out,
        response_format,
        out_dir,
        out_prefix,
        model,
        resolution,
        quality,
        image,
    } = command;
    let response_format = ResponseFormat::parse(&response_format).ok_or_else(|| {
        crate::error::Error::InvalidValue {
            flag: "response-format",
            value: response_format,
        }
    })?;
    if globals.verbose {
        eprintln!(
            "image_gen tool={} command={}",
            crate::infra::imagine::IMAGE_GEN_TOOL_NAME,
            crate::infra::imagine::IMAGINE_COMMAND_NAME
        );
    }
    let result = image::generate(
        ctx,
        image::ImageGenerateOpts {
            request: ImageGenerateRequest {
                prompt,
                model: model.unwrap_or_else(|| ctx.config.default_image_model.clone()),
                aspect_ratio,
                n,
                resolution,
                quality,
                response_format,
                image,
            },
            out,
            out_dir,
            out_prefix,
            dry_run: globals.dry_run,
        },
    )
    .await?;
    let text = if globals.dry_run {
        "dry-run: would call POST /images/generations".to_string()
    } else {
        result
            .images
            .iter()
            .filter_map(|image| image.path.clone().or(image.url.clone()))
            .collect::<Vec<_>>()
            .join("\n")
    };
    emit(output, globals.quiet, text, &result)
}

async fn video_cmd(
    ctx: &AuthContext,
    globals: &Globals,
    output: OutputFormat,
    command: VideoCommand,
) -> Result<()> {
    match command {
        VideoCommand::Generate {
            prompt,
            model,
            image,
            reference_image,
            duration,
            aspect_ratio,
            download,
            no_wait,
            r#async,
            poll_interval,
        } => {
            if globals.verbose {
                eprintln!(
                    "video tool={} command={}",
                    crate::infra::imagine::IMAGE_TO_VIDEO_TOOL_NAME,
                    crate::infra::imagine::IMAGINE_VIDEO_COMMAND_NAME
                );
            }
            let result = video::generate(
                ctx,
                VideoGenerateRequest {
                    prompt,
                    model: model.unwrap_or_else(|| ctx.config.default_video_model.clone()),
                    image,
                    reference_images: reference_image,
                    duration,
                    aspect_ratio,
                    wait: !(no_wait || r#async),
                    poll_interval_secs: poll_interval.unwrap_or(DEFAULT_POLL_INTERVAL_SECS),
                },
                download,
                globals.dry_run,
            )
            .await?;
            let text = if globals.dry_run {
                "dry-run: would call POST /videos/generations".to_string()
            } else {
                result
                    .path
                    .clone()
                    .or(result.url.clone())
                    .unwrap_or(result.request_id.clone())
            };
            emit(output, globals.quiet, text, &result)
        }
        VideoCommand::Task {
            command: VideoTaskCommand::Get { task_id },
        } => {
            let task = video::task_get(ctx, &task_id).await?;
            emit(
                output,
                globals.quiet,
                format!("{} {}", task.request_id, task.status),
                &task,
            )
        }
        VideoCommand::Download { file_id, out } => {
            let result = video::download(ctx, &file_id, &out).await?;
            emit(
                output,
                globals.quiet,
                result.path.clone().unwrap_or(file_id),
                &result,
            )
        }
    }
}

fn config_cmd(globals: &Globals, output: OutputFormat, command: ConfigCommand) -> Result<()> {
    match command {
        ConfigCommand::Show => {
            let config = config_service::load()?;
            emit(
                output,
                globals.quiet,
                format!(
                    "auth_source={} base_url={} timeout={} image={} video={}",
                    config.auth_source.as_str(),
                    config.base_url,
                    config.timeout,
                    config.default_image_model,
                    config.default_video_model
                ),
                &serde_json::json!({
                    "config": config,
                    "environment": xai_grok_env::GrokBuildEnvironment::Production.to_string(),
                }),
            )
        }
        ConfigCommand::Set { key, value } => {
            if globals.dry_run {
                return emit_text(
                    output,
                    globals.quiet,
                    format!("dry-run: would set {key}={value}"),
                );
            }
            let config = config_service::set(&key, &value)?;
            emit(output, globals.quiet, format!("set {key}={value}"), &config)
        }
    }
}
