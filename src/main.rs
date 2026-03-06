mod cli;
mod commands;
mod config;
mod container;
mod error;
mod runtime;
mod template;
mod user;

use clap::Parser;
use colored::Colorize;

use cli::{Cli, Commands};
use commands::create::CreateArgs;
use config::Config;
use container::store::ContainerStore;

fn main() {
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

    if let Commands::Registry { action } = cli.command {
        return match action {
            cli::RegistryAction::List   => commands::registry::list(),
            cli::RegistryAction::Update => commands::registry::update(),
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
        } => {
            commands::create::run(
                &store,
                &cfg,
                CreateArgs {
                    name,
                    distro,
                    template,
                    repo,
                    packages,
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
        Commands::Config { .. } | Commands::Registry { .. } => unreachable!(),
    }

    Ok(())
}
