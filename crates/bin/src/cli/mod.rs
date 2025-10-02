use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "entity-cli",
    disable_help_flag = true,
    disable_help_subcommand = true,
    disable_version_flag = true
)]
#[command(about = "Entity CLI - Global Graph Engine", long_about = None)]
pub struct Cli {
    /// Packs root directory (global; can be placed anywhere)
    #[arg(long, global = true)]
    pub packs: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Emit the graph once as JSON
    Init(InitArgs),

    /// Docs related commands
    Docs(DocsCmd),

    /// UI installation commands
    Ui(UiCmd),

    /// Setup commands
    Setup(SetupCmd),

    /// Bridge commands
    Bridge(BridgeCmd),

    /// Serve minimal HTTP API for agents
    Serve(ServeCmd),
}

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Product/pack name (e.g., entity-auth, microsoft)
    pub product: String,
}

#[derive(Args, Debug)]
pub struct DocsCmd {
    #[command(subcommand)]
    pub command: DocsSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum DocsSubcommand {
    /// Read a doc node content
    Read(DocsReadArgs),
}

#[derive(Args, Debug)]
pub struct DocsReadArgs {
    /// Product/pack name (e.g., entity-auth)
    pub product: String,
    #[arg(long)]
    pub node: String,
}

#[derive(Args, Debug)]
pub struct UiCmd {
    #[command(subcommand)]
    pub command: UiSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum UiSubcommand {
    /// Install UI components
    Install(UiInstallArgs),
}

#[derive(Args, Debug)]
pub struct UiInstallArgs {
    /// Product/pack name (e.g., entity-auth)
    pub product: String,
    #[arg(long, required = false)]
    pub mode: Option<String>,
    #[arg(long, num_args = 1..)]
    pub names: Option<Vec<String>>, // allow omission to distinguish for mode=all
    #[arg(long, default_value = "entityauth:components:install")]
    pub node: String,
}

#[derive(Args, Debug)]
pub struct ServeCmd {
    /// Address to bind (e.g., 127.0.0.1:8787)
    #[arg(long, default_value = "127.0.0.1:8787")]
    pub addr: String,
}

#[derive(Args, Debug)]
pub struct SetupCmd {
    #[command(subcommand)]
    pub command: SetupSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum SetupSubcommand {
    /// Run a setup template (scaffold + copy /entity-auth/*)
    Run(SetupRunArgs),
}

#[derive(Args, Debug)]
pub struct SetupRunArgs {
    /// Product/pack name (e.g., entity-auth)
    pub product: String,
    /// Setup node id (e.g., entityauth:setup:basic)
    #[arg(long)]
    pub node: String,
    /// Workspace directory to operate in (defaults to cwd)
    #[arg(long)]
    pub workspace: Option<String>,
}

#[derive(Args, Debug)]
pub struct BridgeCmd {
    #[command(subcommand)]
    pub command: BridgeSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum BridgeSubcommand {
    /// Scaffold bridge template into workspace
    Scaffold(BridgeScaffoldArgs),
    /// Output process descriptor for bridge runtime
    Start(BridgeStartArgs),
    /// Report bridge runtime status
    Status(BridgeStatusArgs),
    /// Stop bridge runtime
    Stop(BridgeStopArgs),
}

#[derive(Args, Debug)]
pub struct BridgeArgsBase {
    /// Product/pack name (e.g., entity-auth)
    pub product: String,
    /// Bridge node id (e.g., entityauth:bridge:postgres)
    #[arg(long)]
    pub node: String,
    /// Workspace directory to operate in (defaults to cwd)
    #[arg(long)]
    pub workspace: Option<String>,
}

#[derive(Args, Debug)]
pub struct BridgeScaffoldArgs {
    #[command(flatten)]
    pub base: BridgeArgsBase,
}

#[derive(Args, Debug)]
pub struct BridgeStartArgs {
    #[command(flatten)]
    pub base: BridgeArgsBase,
}

#[derive(Args, Debug)]
pub struct BridgeStatusArgs {
    #[command(flatten)]
    pub base: BridgeArgsBase,
}

#[derive(Args, Debug)]
pub struct BridgeStopArgs {
    #[command(flatten)]
    pub base: BridgeArgsBase,
}
