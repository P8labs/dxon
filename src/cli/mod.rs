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
        #[arg(long, short = 'y')]
        trust: bool,
        #[arg(long, value_name = "SHELL")]
        shell: Option<String>,
        #[arg(long, value_name = "MODE")]
        shell_config: Option<String>,
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

    Open {
        name: String,
        #[arg(long, short, value_name = "BINARY")]
        editor: Option<String>,
    },

    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    Show,
    Set { key: String, value: String },
}

#[derive(Subcommand)]
pub enum TemplateAction {
    List,
    Search { keyword: String },
    Refresh,
}
