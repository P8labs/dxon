use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::collections::HashMap;

use crate::config::Config;
use crate::container::meta::ContainerMeta;
use crate::container::store::ContainerStore;
use crate::error::DxonError;
use crate::runtime::bootstrap::{bootstrap, Distro};
use crate::runtime::host::HostInfo;
use crate::runtime::nspawn::{install_packages_with_fallback, require_nspawn, run_command};
use crate::runtime::packages::translate_list;
use crate::template;
use crate::template::builtin;
use crate::template::spec::{DxTemplate, Step};

pub struct CreateArgs {
    pub name: Option<String>,
    pub distro: Option<String>,
    pub template: Option<String>,
    pub repo: Option<String>,
    pub packages: Vec<String>,
    pub trust: bool,
}

pub fn run(store: &ContainerStore, cfg: &Config, args: CreateArgs) -> Result<()> {
    require_nspawn()?;

    let theme = ColorfulTheme::default();
    let host = HostInfo::detect();

    let name: String = match args.name {
        Some(n) => n,
        None => Input::with_theme(&theme)
            .with_prompt("Container name")
            .interact_text()?,
    };

    if store.exists(&name) {
        return Err(DxonError::ContainerExists(name).into());
    }

    let distro_choices = &["arch", "debian", "alpine"];

    let distro_str: String = match args.distro {
        Some(d) => d,
        None => {
            let default_idx = cfg
                .default_distro
                .as_deref()
                .and_then(|d| distro_choices.iter().position(|&c| c == d))
                .unwrap_or(0);

            let idx = Select::with_theme(&theme)
                .with_prompt("Base distribution")
                .items(distro_choices)
                .default(default_idx)
                .interact()?;
            distro_choices[idx].to_string()
        }
    };
    let distro = Distro::parse(&distro_str)?;

    let (bootstrap_tool, hint) = host.bootstrap_tool_for(&distro_str);
    if !tool_available(bootstrap_tool) {
        return Err(DxonError::MissingTool {
            tool: bootstrap_tool.to_string(),
            hint: format!("on {}: {hint}", host.pretty_name),
        }
        .into());
    }

    let (tmpl, tmpl_name): (Option<DxTemplate>, Option<String>) = match args.template {
        Some(ref t) => {
            let (loaded, source) = template::resolve(t, cfg.effective_registry_url())?;
            check_trust(&source, args.trust, &theme)?;
            (Some(loaded), Some(t.clone()))
        }
        None => {
            let mut options: Vec<String> = builtin::list_descriptions()
                .iter()
                .map(|(n, d)| format!("{n:8}  {d}"))
                .collect();
            options.push("none".to_string());

            let default_tmpl_idx = cfg
                .default_template
                .as_deref()
                .and_then(|t| builtin::BUILTIN_NAMES.iter().position(|&b| b == t))
                .unwrap_or(options.len() - 1);

            let idx = Select::with_theme(&theme)
                .with_prompt("Template (optional)")
                .items(&options)
                .default(default_tmpl_idx)
                .interact()?;

            if idx < builtin::BUILTIN_NAMES.len() {
                let bname = builtin::BUILTIN_NAMES[idx];
                let (loaded, source) = template::resolve(bname, cfg.effective_registry_url())?;
                check_trust(&source, args.trust, &theme)?;
                (Some(loaded), Some(bname.to_string()))
            } else {
                (None, None)
            }
        }
    };

    let mut answers: HashMap<String, String> = HashMap::new();
    if let Some(ref t) = tmpl {
        for prompt in &t.prompts {
            let idx = Select::with_theme(&theme)
                .with_prompt(&prompt.question)
                .items(&prompt.options)
                .default(default_idx(&prompt.options, &prompt.default))
                .interact()?;
            answers.insert(prompt.id.clone(), prompt.options[idx].clone());
        }
    }

    let repo: Option<String> = match args.repo {
        Some(r) => Some(r),
        None => {
            let want = Confirm::with_theme(&theme)
                .with_prompt("Clone a Git repository into the container?")
                .default(false)
                .interact()?;
            if want {
                Some(
                    Input::with_theme(&theme)
                        .with_prompt("Repository URL")
                        .interact_text()?,
                )
            } else {
                None
            }
        }
    };

    let container_dir = store.container_dir(&name);
    let rootfs_dir = store.rootfs_dir(&name);

    println!();
    println!("  {}", "Creating container".bold());
    println!("  {:<16} {}", "name:".dimmed(), name.cyan());
    println!("  {:<16} {}", "distro:".dimmed(), distro_str.cyan());
    if let Some(ref t) = tmpl_name {
        println!("  {:<16} {}", "template:".dimmed(), t.cyan());
    }
    if let Some(ref r) = repo {
        println!("  {:<16} {}", "repo:".dimmed(), r.cyan());
    }
    println!(
        "  {:<16} {}",
        "storage:".dimmed(),
        container_dir.display().to_string().cyan()
    );
    println!("  {:<16} {}", "host:".dimmed(), host.pretty_name.dimmed());
    println!();

    store.create_dirs(&name)?;
    let rootfs = store.rootfs_dir(&name);

    println!("{} bootstrapping {} rootfs…", "→".cyan(), distro_str.bold());
    bootstrap(&distro, &rootfs).map_err(|e| {
        let _ = store.remove(&name);
        e
    })?;

    let mut installed_packages: Vec<String> = Vec::new();

    if let Some(ref t) = tmpl {
        let base_pkgs: Vec<String> = if !t.base.packages_by_distro.is_empty() {
            t.base
                .packages_by_distro
                .get(&distro_str)
                .cloned()
                .unwrap_or_default()
        } else {
            translate_list(&t.base.packages, &distro_str)
        };

        if !base_pkgs.is_empty() {
            println!(
                "{} installing base packages: {}",
                "→".cyan(),
                base_pkgs.join(", ").bold()
            );
            install_packages_with_fallback(&rootfs, &base_pkgs, &distro_str, &HashMap::new())?;
            installed_packages.extend(base_pkgs);
        }

        for step in &t.steps {
            if !step_applies(step, &distro_str, &answers) {
                continue;
            }

            println!("{} {}…", "→".cyan(), step.name.bold());

            if !step.tools.is_empty() {
                let pkgs = translate_list(&step.tools, &distro_str);
                println!("  {} packages: {}", "↳".dimmed(), pkgs.join(", ").dimmed());
                install_packages_with_fallback(&rootfs, &pkgs, &distro_str, &t.runtime.env)?;
                installed_packages.extend(pkgs);
            }

            for cmd in &step.commands {
                run_command(&rootfs, cmd, &t.runtime.env)?;
            }
        }

        for cmd in &t.runtime.commands {
            run_command(&rootfs, cmd, &t.runtime.env)?;
        }
    }

    if !args.packages.is_empty() {
        let translated = translate_list(&args.packages, &distro_str);
        println!(
            "{} installing extra packages: {}",
            "→".cyan(),
            translated.join(", ").bold()
        );
        install_packages_with_fallback(&rootfs, &translated, &distro_str, &HashMap::new())?;
        installed_packages.extend(translated);
    }

    if let Some(ref url) = repo {
        println!("{} cloning {}…", "→".cyan(), url.bold());
        run_command(
            &rootfs,
            &format!("git clone {url} /workspace"),
            &HashMap::new(),
        )
        .map_err(|_| DxonError::GitCloneFailed(url.clone()))?;
    }

    let mut meta = ContainerMeta::new(&name, &distro_str, rootfs_dir.to_str().unwrap());
    meta.template = tmpl_name;
    meta.packages = installed_packages;
    meta.repo = repo;
    if let Some(ref t) = tmpl {
        meta.config.env = t.runtime.env.clone();
    }
    store.save_meta(&meta)?;

    println!();
    println!(
        "{} container {} is ready.",
        "✓".green().bold(),
        name.cyan().bold()
    );
    println!("  rootfs:  {}", rootfs.display().to_string().dimmed());
    println!("  enter:   {}", format!("dxon enter {name}").bold());
    println!();

    Ok(())
}

fn default_idx(options: &[String], default: &Option<String>) -> usize {
    let Some(d) = default else { return 0 };
    options.iter().position(|o| o == d).unwrap_or(0)
}

fn step_applies(step: &Step, distro: &str, answers: &HashMap<String, String>) -> bool {
    if let Some(ref d) = step.distro {
        if d != distro {
            return false;
        }
    }
    for (k, v) in &step.when {
        if answers.get(k).map(|s| s.as_str()) != Some(v.as_str()) {
            return false;
        }
    }
    true
}

fn tool_available(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_trust(
    source: &template::TemplateSource,
    trust_flag: bool,
    theme: &dialoguer::theme::ColorfulTheme,
) -> anyhow::Result<()> {
    if source.is_trusted() {
        println!(
            "  {} template source: {} {}",
            "✓".green(),
            source.label().dimmed(),
            "(trusted)".green()
        );
        return Ok(());
    }

    eprintln!();
    eprintln!(
        "  {} {}",
        "warning:".yellow().bold(),
        "template is from an untrusted source".bold()
    );
    eprintln!("  {:<12} {}", "source:".dimmed(), source.label().yellow());
    eprintln!("  {:<12} {}", "kind:".dimmed(), source.kind());
    eprintln!(
        "  {}",
        "Templates may execute arbitrary commands during container setup.".dimmed()
    );
    eprintln!("  {}", "Only proceed if you trust this source.".dimmed());
    eprintln!();

    if trust_flag {
        println!(
            "  {} proceeding with untrusted template (--trust passed)",
            "→".yellow()
        );
        return Ok(());
    }

    let confirmed = Confirm::with_theme(theme)
        .with_prompt("Proceed with this template?")
        .default(false)
        .interact()?;

    if !confirmed {
        anyhow::bail!("aborted — template source not trusted");
    }

    Ok(())
}
