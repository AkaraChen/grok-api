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
    /// Request timeout (default: 300)
    #[arg(long, global = true)]
    pub timeout: Option<u64>,
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
    /// Image generation (generate, model list)
    Image {
        #[command(subcommand)]
        command: ImageCommand,
    },
    /// Video generation (generate, task get, download, voice list)
    Video {
        #[command(subcommand)]
        command: VideoCommand,
    },
    /// Web search (query)
    Search {
        #[command(subcommand)]
        command: SearchCommand,
    },
    /// MCP server (stdio or streamable HTTP)
    Mcp {
        #[command(subcommand)]
        command: Option<McpCommand>,
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
        /// Aspect ratio: auto (default), 1:1, 16:9, 9:16, 4:3, 3:4, 3:2, 2:3, 2:1, 1:2, 21:9, 19.5:9
        #[arg(long)]
        aspect_ratio: Option<String>,
        /// Number of images to generate (default: 1)
        #[arg(long, default_value_t = 1)]
        n: u32,
        /// Save image to exact file path (single image only)
        #[arg(long)]
        out: Option<PathBuf>,
        /// Response format: url|base64 (default url). Wire names: url, b64_json
        #[arg(long, default_value = "url")]
        response_format: String,
        /// Download images to directory
        #[arg(long)]
        out_dir: Option<PathBuf>,
        /// Filename prefix (default: image)
        #[arg(long, default_value = "image")]
        out_prefix: String,
        /// Image model ID (default: grok-imagine-image-2.0). See `grok-api image model list`. Examples: grok-imagine-image-2.0, grok-imagine-image-quality
        #[arg(long)]
        model: Option<String>,
        /// Output resolution: 1k (default), 2k
        #[arg(long)]
        resolution: Option<String>,
        /// Quality: low, medium, auto. Imagine accepts auto; high is not supported
        #[arg(long, value_parser = ["low", "medium", "auto"])]
        quality: Option<String>,
        /// Source image path or URL for image editing
        #[arg(long)]
        image: Option<String>,
    },
    /// Query Imagine image models
    Model {
        #[command(subcommand)]
        command: ImageModelCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum ImageModelCommand {
    /// List Imagine image models from GET /models
    List,
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
        /// Resolution: 480p (API default if omitted), 720p, 1080p. Same flag for text-to-video and image-to-video
        #[arg(long)]
        resolution: Option<String>,
        /// Preset TTS voice_id (repeatable). Tag speakers as <AUDIO_0>, <AUDIO_1>, <AUDIO_2>. Examples: eve, ara, leo, rex. See `grok-api video voice list`
        #[arg(long, alias = "reference-audio")]
        voice: Vec<String>,
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
    /// Query TTS voices for reference-to-video
    Voice {
        #[command(subcommand)]
        command: VideoVoiceCommand,
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
pub enum VideoVoiceCommand {
    /// List TTS voices from GET /tts/voices
    List,
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
pub enum SearchCommand {
    /// Search the web via POST /responses with the official web_search tool
    Query {
        /// Search query
        #[arg(long)]
        query: String,
        /// Restrict results to this domain (repeatable)
        #[arg(long)]
        allowed_domain: Vec<String>,
        /// Exclude this domain (repeatable)
        #[arg(long)]
        excluded_domain: Vec<String>,
        /// Responses model used by the official web_search tool
        #[arg(long, env = "GROK_WEB_SEARCH_MODEL")]
        model: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum McpCommand {
    /// Serve MCP over stdio (default when `grok-api mcp` has no command)
    Stdio,
    /// Serve MCP over streamable HTTP
    Http {
        /// Bind address (default: 127.0.0.1:3920)
        #[arg(long, default_value = "127.0.0.1:3920")]
        bind: String,
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
  image      Image generation (generate, model list)
  video      Video generation (generate, task get, download, voice list)
  search     Web search (query)
  mcp        MCP server (stdio, http)
  config     CLI configuration (show, set)

Global Flags:
  --api-key <key>        API key (overrides all other auth)
  --from-grok-cli        Read the official Grok CLI key from ~/.grok
  --base-url <url>       API base URL (overrides config)
  --output <format>      Output format: text, json
  --quiet                Suppress non-essential output
  --timeout <seconds>    Request timeout (default: 300)
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
                command:
                    ImageCommand::Generate {
                        prompt,
                        aspect_ratio,
                        n,
                        ..
                    },
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
        let cli =
            Cli::try_parse_from(["grok-api", "video", "task", "get", "--task-id", "abc"]).unwrap();
        match cli.resource {
            Some(Resource::Video {
                command:
                    VideoCommand::Task {
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

    fn image_generate_help() -> String {
        let mut cmd = Cli::command();
        cmd.find_subcommand_mut("image")
            .unwrap()
            .find_subcommand_mut("generate")
            .unwrap()
            .render_long_help()
            .to_string()
    }

    fn video_generate_help() -> String {
        let mut cmd = Cli::command();
        cmd.find_subcommand_mut("video")
            .unwrap()
            .find_subcommand_mut("generate")
            .unwrap()
            .render_long_help()
            .to_string()
    }

    #[test]
    fn image_generate_help_covers_defaults() {
        let help = image_generate_help();
        assert!(help.contains("low"));
        assert!(help.contains("medium"));
        assert!(help.contains("auto"));
        assert!(help.contains("1k"));
        assert!(help.contains("2k"));
        assert!(help.contains("url|base64") || help.contains("url") && help.contains("base64"));
        assert!(help.contains("b64_json"));
        assert!(help.contains("image model list"));
        assert!(help.contains("1:1"));
        assert!(help.contains("16:9"));
        assert!(help.contains("21:9"));
    }

    #[test]
    fn video_generate_help_covers_resolution_and_voice() {
        let help = video_generate_help();
        assert!(help.contains("480p"));
        assert!(help.contains("720p"));
        assert!(help.contains("1080p"));
        assert!(help.contains("--voice") || help.contains("voice"));
        assert!(help.contains("<AUDIO_0>"));
        assert!(help.contains("video voice list"));
    }

    #[test]
    fn rejects_image_quality_high() {
        let err = Cli::try_parse_from([
            "grok-api",
            "image",
            "generate",
            "--prompt",
            "a cat",
            "--quality",
            "high",
        ])
        .unwrap_err();
        let text = err.to_string();
        assert!(text.contains("high"));
        assert!(text.contains("low") && text.contains("medium") && text.contains("auto"));
    }

    #[test]
    fn parses_image_model_list() {
        let cli = Cli::try_parse_from(["grok-api", "image", "model", "list"]).unwrap();
        assert!(matches!(
            cli.resource,
            Some(Resource::Image {
                command: ImageCommand::Model {
                    command: ImageModelCommand::List
                }
            })
        ));
    }

    #[test]
    fn parses_video_voice_and_resolution() {
        let cli = Cli::try_parse_from([
            "grok-api",
            "video",
            "generate",
            "--prompt",
            "hello <AUDIO_0>",
            "--voice",
            "eve",
            "--resolution",
            "720p",
            "--async",
        ])
        .unwrap();
        match cli.resource {
            Some(Resource::Video {
                command:
                    VideoCommand::Generate {
                        voice,
                        resolution,
                        r#async,
                        ..
                    },
            }) => {
                assert_eq!(voice, ["eve"]);
                assert_eq!(resolution.as_deref(), Some("720p"));
                assert!(r#async);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parses_video_voice_list() {
        let cli = Cli::try_parse_from(["grok-api", "video", "voice", "list"]).unwrap();
        assert!(matches!(
            cli.resource,
            Some(Resource::Video {
                command: VideoCommand::Voice {
                    command: VideoVoiceCommand::List
                }
            })
        ));
    }

    #[test]
    fn parses_mcp_stdio_default() {
        let cli = Cli::try_parse_from(["grok-api", "mcp"]).unwrap();
        assert!(matches!(
            cli.resource,
            Some(Resource::Mcp { command: None })
        ));
    }

    #[test]
    fn parses_mcp_http_bind() {
        let cli =
            Cli::try_parse_from(["grok-api", "mcp", "http", "--bind", "127.0.0.1:4000"]).unwrap();
        match cli.resource {
            Some(Resource::Mcp {
                command: Some(McpCommand::Http { bind }),
            }) => assert_eq!(bind, "127.0.0.1:4000"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parses_search_query() {
        let cli = Cli::try_parse_from([
            "grok-api",
            "search",
            "query",
            "--query",
            "rust async",
            "--allowed-domain",
            "docs.rs",
            "--allowed-domain",
            "tokio.rs",
            "--model",
            "grok-4.6",
        ])
        .unwrap();
        match cli.resource {
            Some(Resource::Search {
                command:
                    SearchCommand::Query {
                        query,
                        allowed_domain,
                        excluded_domain,
                        model,
                    },
            }) => {
                assert_eq!(query, "rust async");
                assert_eq!(allowed_domain, ["docs.rs", "tokio.rs"]);
                assert!(excluded_domain.is_empty());
                assert_eq!(model.as_deref(), Some("grok-4.6"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
