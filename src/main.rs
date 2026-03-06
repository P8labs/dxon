mod cli;
mod commands;
mod config;
mod container;
mod error;
mod runtime;
mod shell_config;
mod template;
mod user;

use clap::Parser;
use colored::Colorize;

use cli::{Cli, Commands};
use commands::create::CreateArgs;
use config::Config;
use container::store::ContainerStore;

fn main() {
    pub const VERSION: &str = env!("DXON_VERSION");
    println!("dXon {}", VERSION);

    if let Err(e) = run() {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut cfg = Config::load()?;

    if let Commands::Config { action } = cli.command {
        return match action {
            cli::ConfigAction::Show => commands::config::show(&cfg),
            cli::ConfigAction::Set { key, value } => commands::config::set(&mut cfg, &key, &value),
        };
    }

    if let Commands::Template { action } = cli.command {
        let registry_url = cfg.effective_registry_url().to_string();
        return match action {
            cli::TemplateAction::List => commands::registry::list(&registry_url),
            cli::TemplateAction::Search { keyword } => {
                commands::registry::search(&keyword, &registry_url)
            }
            cli::TemplateAction::Refresh => commands::registry::refresh(&registry_url),
        };
    }

    let containers_path = cfg.containers_dir(cli.dir.as_deref())?;
    let store = ContainerStore::new(containers_path)?;

    match cli.command {
        Commands::Create {
            name,
            distro,
            template,
            repo,
            packages,
            trust,
            shell,
            shell_config,
        } => {
            commands::create::run(
                &store,
                &mut cfg,
                CreateArgs {
                    name,
                    distro,
                    template,
                    repo,
                    packages,
                    trust,
                    shell,
                    shell_config,
                },
            )?;
        }
        Commands::Delete { name, force } => {
            commands::delete::run(&store, &name, force)?;
        }
        Commands::List => {
            commands::list::run(&store)?;
        }
        Commands::Info { name } => {
            commands::info::run(&store, &name)?;
        }
        Commands::Enter { name, cmd } => {
            commands::enter::run(&store, &name, &cmd)?;
        }
        Commands::Open { name, editor } => {
            commands::open::run(&store, &mut cfg, &name, editor.as_deref())?;
        }
        Commands::Config { .. } | Commands::Template { .. } => unreachable!(),
    }

    Ok(())
}
