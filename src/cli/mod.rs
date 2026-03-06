use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "dxon")]
#[command(version)]
#[command(about = "Lightweight development container manager")]
#[command(
    long_about = "dxon creates and manages isolated development environments\nbuilt on pacstrap, debootstrap, and systemd-nspawn."
)]
pub struct Cli {
    #[arg(long, global = true, env = "DXON_DIR", value_name = "PATH")]
    pub dir: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Create {
        name: Option<String>,
        #[arg(long, short, value_name = "DISTRO")]
        distro: Option<String>,
        #[arg(long, short, value_name = "TEMPLATE")]
        template: Option<String>,
        #[arg(long, short, value_name = "URL")]
        repo: Option<String>,
        #[arg(long, short, num_args = 1.., value_name = "PKG")]
        packages: Vec<String>,
    },

    Delete {
        name: String,
        #[arg(long, short)]
        force: bool,
    },

    List,
    Info {
        name: String,
    },

    Enter {
        name: String,
        #[arg(last = true)]
        cmd: Vec<String>,
    },
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    Registry {
        #[command(subcommand)]
        action: RegistryAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    Show,
    Set { key: String, value: String },
}

#[derive(Subcommand)]
pub enum RegistryAction {
    /// List available templates (cached + built-in)
    List,
    /// Download/refresh templates from github.com/P8labs/dxon-registry
    Update,
}
