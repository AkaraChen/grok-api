use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::model::auth::AuthSource;
use crate::model::config::OutputFormat;

#[derive(Parser, Debug)]
#[command(
    name = "grok-api",
    bin_name = "grok-api",
    version,
    about = "CLI for the Grok / xAI API",
    override_usage = "grok-api <resource> <command> [flags]",
    disable_help_subcommand = true,
    next_line_help = false
)]
pub struct Cli {
    #[command(flatten)]
    pub globals: Globals,
    #[command(subcommand)]
    pub resource: Option<Resource>,
}

#[derive(Args, Debug, Clone)]
pub struct Globals {
    /// API key (overrides all other auth)
    #[arg(long, global = true, env = "XAI_API_KEY")]
    pub api_key: Option<String>,
    /// API base URL (overrides config)
    #[arg(long, global = true)]
    pub base_url: Option<String>,
    /// Output format: text, json
    #[arg(long, global = true, value_enum)]
    pub output: Option<CliOutput>,
    /// Suppress non-essential output
    #[arg(long, global = true)]
    pub quiet: bool,
    /// Print HTTP request/response details
    #[arg(long, global = true)]
    pub verbose: bool,
    /// Request timeout (default: 300)
    #[arg(long, global = true)]
    pub timeout: Option<u64>,
    /// Disable ANSI colors and spinners
    #[arg(long, global = true)]
    pub no_color: bool,
    /// Show what would happen without executing
    #[arg(long, global = true)]
    pub dry_run: bool,
    /// Disable interactive prompts (CI/agent mode)
    #[arg(long, global = true)]
    pub non_interactive: bool,
    /// Read credentials from the official Grok CLI home (~/.grok)
    #[arg(long, global = true)]
    pub from_grok_cli: bool,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum CliOutput {
    Text,
    Json,
}

impl From<CliOutput> for OutputFormat {
    fn from(value: CliOutput) -> Self {
        match value {
            CliOutput::Text => Self::Text,
            CliOutput::Json => Self::Json,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Resource {
    /// Authentication (login, status, refresh, logout)
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// Image generation (generate)
    Image {
        #[command(subcommand)]
        command: ImageCommand,
    },
    /// Video generation (generate, task get, download)
    Video {
        #[command(subcommand)]
        command: VideoCommand,
    },
    /// CLI configuration (show, set)
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuthCommand {
    /// Authenticate via official Grok login, Grok CLI key, or API key
    Login {
        /// Skip the menu and save this API key directly
        #[arg(long)]
        api_key: Option<String>,
        /// Use Grok OAuth via auth.x.ai (writes ~/.grok-api)
        #[arg(long)]
        oauth: bool,
        /// Use device-code authentication for headless/remote environments
        #[arg(long, alias = "device-code")]
        device_auth: bool,
        /// Read the official Grok CLI key from ~/.grok
        #[arg(long)]
        from_grok_cli: bool,
    },
    /// Show current authentication state
    Status,
    /// Re-run official Grok login into ~/.grok-api
    Refresh {
        #[arg(long, alias = "device-code")]
        device_auth: bool,
    },
    /// Clear stored credentials in ~/.grok-api
    Logout {
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ImageCommand {
    /// Generate images (grok-imagine-image)
    Generate {
        /// Image description
        #[arg(long)]
        prompt: String,
        /// Aspect ratio (e.g. 16:9, 1:1)
        #[arg(long)]
        aspect_ratio: Option<String>,
        /// Number of images to generate (default: 1)
        #[arg(long, default_value_t = 1)]
        n: u32,
        /// Save image to exact file path (single image only)
        #[arg(long)]
        out: Option<PathBuf>,
        /// Response format: url (download), base64 (embed)
        #[arg(long, default_value = "url")]
        response_format: String,
        /// Download images to directory
        #[arg(long)]
        out_dir: Option<PathBuf>,
        /// Filename prefix (default: image)
        #[arg(long, default_value = "image")]
        out_prefix: String,
        /// Image model ID
        #[arg(long)]
        model: Option<String>,
        /// Output resolution: 1k, 2k
        #[arg(long)]
        resolution: Option<String>,
        /// Quality: low, medium
        #[arg(long)]
        quality: Option<String>,
        /// Source image path or URL for image editing
        #[arg(long)]
        image: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum VideoCommand {
    /// Generate a video
    Generate {
        /// Video description
        #[arg(long)]
        prompt: String,
        /// Model ID
        #[arg(long)]
        model: Option<String>,
        /// Input image for image-to-video (local path or URL)
        #[arg(long)]
        image: Option<String>,
        /// Reference image (repeatable)
        #[arg(long)]
        reference_image: Vec<String>,
        /// Output duration in seconds
        #[arg(long)]
        duration: Option<u32>,
        /// Aspect ratio: 16:9, 9:16, 1:1, 4:3, 3:4
        #[arg(long, alias = "ratio")]
        aspect_ratio: Option<String>,
        /// Save video to file on completion
        #[arg(long)]
        download: Option<PathBuf>,
        /// Return task ID immediately without waiting
        #[arg(long)]
        no_wait: bool,
        /// Return task ID immediately (agent/CI mode, same as --no-wait)
        #[arg(long)]
        r#async: bool,
        /// Polling interval when waiting (default: 5)
        #[arg(long)]
        poll_interval: Option<u64>,
    },
    /// Query or download video tasks
    Task {
        #[command(subcommand)]
        command: VideoTaskCommand,
    },
    /// Download a completed video by request ID or URL
    Download {
        /// File / request ID to download
        #[arg(long)]
        file_id: String,
        /// Output file path
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
pub enum VideoTaskCommand {
    /// Query video task status
    Get {
        /// Video generation task ID
        #[arg(long)]
        task_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Display current configuration
    Show,
    /// Set a config value
    Set {
        /// Config key
        #[arg(long)]
        key: String,
        /// Value to set
        #[arg(long)]
        value: String,
    },
}

pub fn print_root_help() {
    println!(
        r#"
 ██████╗ ██████╗  ██████╗ ██╗  ██╗
██╔════╝ ██╔══██╗██╔═══██╗██║ ██╔╝
██║  ███╗██████╔╝██║   ██║█████╔╝
██║   ██║██╔══██╗██║   ██║██╔═██╗
╚██████╔╝██║  ██║╚██████╔╝██║  ██╗
 ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝
               A P I

Usage: grok-api <resource> <command> [flags]

Resources:
  auth       Authentication (login, status, refresh, logout)
  image      Image generation (generate)
  video      Video generation (generate, task get, download)
  config     CLI configuration (show, set)

Global Flags:
  --api-key <key>        API key (overrides all other auth)
  --from-grok-cli        Read the official Grok CLI key from ~/.grok
  --base-url <url>       API base URL (overrides config)
  --output <format>      Output format: text, json
  --quiet                Suppress non-essential output
  --verbose              Print HTTP request/response details
  --timeout <seconds>    Request timeout (default: 300)
  --no-color             Disable ANSI colors and spinners
  --dry-run              Show what would happen without executing
  --non-interactive      Disable interactive prompts (CI/agent mode)
  --version              Print version and exit
  --help                 Show help

Getting Help:
  Add --help after any command to see its full list of options, defaults,
  and usage examples. For example: grok-api image generate --help
"#
    );
}

pub fn source_from_globals(globals: &Globals) -> Option<AuthSource> {
    globals.from_grok_cli.then_some(AuthSource::GrokCli)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn parses_image_generate() {
        let cli = Cli::try_parse_from([
            "grok-api",
            "image",
            "generate",
            "--prompt",
            "a cat",
            "--aspect-ratio",
            "16:9",
        ])
        .unwrap();
        match cli.resource {
            Some(Resource::Image {
                command: ImageCommand::Generate { prompt, aspect_ratio, n, .. },
            }) => {
                assert_eq!(prompt, "a cat");
                assert_eq!(aspect_ratio.as_deref(), Some("16:9"));
                assert_eq!(n, 1);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parses_video_task_get() {
        let cli = Cli::try_parse_from([
            "grok-api",
            "video",
            "task",
            "get",
            "--task-id",
            "abc",
        ])
        .unwrap();
        match cli.resource {
            Some(Resource::Video {
                command: VideoCommand::Task {
                    command: VideoTaskCommand::Get { task_id },
                },
            }) => assert_eq!(task_id, "abc"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn clap_debug_assert() {
        Cli::command().debug_assert();
    }
}
