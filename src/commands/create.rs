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
use crate::runtime::nspawn::{
    ensure_container_user, install_packages_with_fallback, require_nspawn, run_command,
};
use crate::runtime::packages::translate_list;
use crate::template;
use crate::template::builtin;
use crate::template::spec::{DxTemplate, Step};
use crate::user;

pub struct CreateArgs {
    pub name: Option<String>,
    pub distro: Option<String>,
    pub template: Option<String>,
    pub repo: Option<String>,
    pub packages: Vec<String>,
    pub trust: bool,
    pub shell: Option<String>,
    pub shell_config: Option<String>,
}

pub fn run(store: &ContainerStore, cfg: &mut Config, args: CreateArgs) -> Result<()> {
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
    if !user::command_available(bootstrap_tool) {
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

    const SHELL_OPTIONS: &[&str] = &["bash", "zsh", "fish", "none (distro default)"];

    let chosen_shell: Option<String> = if let Some(ref s) = args.shell {
        match s.as_str() {
            "bash" | "zsh" | "fish" => Some(s.clone()),
            other => anyhow::bail!("unsupported shell '{other}'\n  valid values: bash, zsh, fish"),
        }
    } else {
        let default_shell_idx = cfg
            .default_shell
            .as_deref()
            .and_then(|s| SHELL_OPTIONS.iter().position(|&c| c == s))
            .unwrap_or(0);

        let idx = Select::with_theme(&theme)
            .with_prompt("Shell")
            .items(SHELL_OPTIONS)
            .default(default_shell_idx)
            .interact()?;

        if idx < 3 {
            Some(SHELL_OPTIONS[idx].to_string())
        } else {
            None
        }
    };

    const CONFIG_OPTIONS: &[&str] = &[
        "no — keep container independent",
        "copy — bake host config into container once",
        "bind — mount host config live on every enter",
    ];

    let chosen_shell_config: Option<String> = if let Some(ref mode) = args.shell_config {
        Some(mode.clone())
    } else if chosen_shell.is_some() {
        let default_idx = cfg
            .copy_shell_config
            .as_deref()
            .and_then(|m| match m {
                "copy" => Some(1usize),
                "bind" => Some(2usize),
                _ => Some(0usize),
            })
            .unwrap_or(0);

        let idx = Select::with_theme(&theme)
            .with_prompt("Copy shell config from host?")
            .items(CONFIG_OPTIONS)
            .default(default_idx)
            .interact()?;

        match idx {
            1 => Some("copy".to_string()),
            2 => Some("bind".to_string()),
            _ => None,
        }
    } else {
        None
    };

    let container_dir = store.container_dir(&name);
    let rootfs_dir = store.rootfs_dir(&name);

    println!();
    println!("  {}", "Creating container".bold());
    println!("  {:<16} {}", "name:".dimmed(), name.cyan());
    println!("  {:<16} {}", "distro:".dimmed(), distro_str.cyan());
    if let Some(ref s) = chosen_shell {
        println!("  {:<16} {}", "shell:".dimmed(), s.cyan());
    }
    if let Some(ref m) = chosen_shell_config {
        println!("  {:<16} {}", "shell config:".dimmed(), m.cyan());
    }
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

    let host_user = user::detect_host_user();

    let provision_result = provision(
        &rootfs,
        &distro_str,
        &distro,
        tmpl.as_ref(),
        &answers,
        &args.packages,
        chosen_shell.as_deref(),
        repo.as_deref(),
        &host_user,
    );

    if let Err(e) = provision_result {
        eprintln!(
            "{} creation failed, cleaning up orphaned container…",
            "!".red()
        );
        if let Err(rm_err) = store.remove(&name) {
            eprintln!("{} cleanup also failed: {rm_err}", "warn:".yellow());
        }
        return Err(e);
    }

    let (installed_packages, final_repo) = provision_result.expect("checked above");

    let mut meta = ContainerMeta::new(&name, &distro_str, rootfs_dir.to_str().unwrap());
    meta.template = tmpl_name;
    meta.packages = installed_packages;
    meta.repo = final_repo;
    meta.config.shell = chosen_shell.clone();
    if let Some(ref t) = tmpl {
        meta.config.env = t.runtime.env.clone();
    }

    if host_user.uid != 0 {
        meta.config.container_user = Some(host_user.username.clone());
        meta.config.container_uid = Some(host_user.uid);
        meta.config.container_gid = Some(host_user.gid);
        meta.config.workspace_dir = Some("/workspace".to_string());
    }

    if let Some(ref mode_str) = chosen_shell_config {
        let mode = crate::shell_config::ShellConfigMode::parse(mode_str).map_err(|e| {
            let _ = store.remove(&name);
            e
        })?;
        let host_home = user::resolve_home();
        let shell_name = chosen_shell.as_deref().unwrap_or("bash");

        // Resolve the container user's home directory (e.g. /home/priyanshu or /root).
        let container_home_abs = if host_user.uid == 0 {
            std::path::PathBuf::from("/root")
        } else {
            std::path::PathBuf::from(format!("/home/{}", host_user.username))
        };

        match mode {
            crate::shell_config::ShellConfigMode::Copy => {
                crate::shell_config::apply_copy(
                    &rootfs,
                    &host_home,
                    shell_name,
                    &container_home_abs,
                )
                .map_err(|e| {
                    let _ = store.remove(&name);
                    e
                })?;
                meta.config.shell_config_mode = Some("copy".to_string());
            }
            crate::shell_config::ShellConfigMode::Bind => {
                let bind_args =
                    crate::shell_config::bind_args(&host_home, shell_name, &container_home_abs);
                meta.config.extra_args.extend(bind_args);
                meta.config.shell_config_mode = Some("bind".to_string());
            }
        }
    }

    store.save_meta(&meta)?;

    let mut prefs_changed = false;
    if args.shell.is_none() {
        if let Some(ref shell) = chosen_shell {
            cfg.default_shell = Some(shell.clone());
            prefs_changed = true;
        }
    }
    if args.shell_config.is_none() {
        let new_mode = chosen_shell_config.clone();
        if new_mode != cfg.copy_shell_config {
            cfg.copy_shell_config = new_mode;
            prefs_changed = true;
        }
    }
    if prefs_changed {
        if let Err(e) = cfg.save() {
            eprintln!("{} could not save preferences: {e}", "warn:".yellow());
        }
    }

    println!();
    println!(
        "{} container {} is ready.",
        "✓".green().bold(),
        name.cyan().bold()
    );
    println!("  rootfs:  {}", rootfs.display().to_string().dimmed());
    if host_user.uid != 0 {
        println!(
            "  user:    {} (uid={}, gid={})",
            host_user.username.dimmed(),
            host_user.uid,
            host_user.gid
        );
        println!("  workdir: {}", "/workspace".dimmed());
    }
    if let Some(ref shell) = chosen_shell {
        println!("  shell:   {}", shell.dimmed());
    }
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

fn provision(
    rootfs: &std::path::Path,
    distro_str: &str,
    distro: &Distro,
    tmpl: Option<&DxTemplate>,
    answers: &HashMap<String, String>,
    extra_packages: &[String],
    chosen_shell: Option<&str>,
    repo: Option<&str>,
    host_user: &user::HostUser,
) -> Result<(Vec<String>, Option<String>)> {
    println!("{} bootstrapping {} rootfs…", "→".cyan(), distro_str.bold());
    bootstrap(distro, rootfs)?;

    // Create a matching user/group inside the container so that file
    // ownership inside /workspace matches the host user.
    ensure_container_user(rootfs, &host_user.username, host_user.uid, host_user.gid)?;

    let mut installed_packages: Vec<String> = Vec::new();

    if let Some(t) = tmpl {
        let base_pkgs: Vec<String> = if !t.base.packages_by_distro.is_empty() {
            t.base
                .packages_by_distro
                .get(distro_str)
                .cloned()
                .unwrap_or_default()
        } else {
            translate_list(&t.base.packages, distro_str)
        };

        if !base_pkgs.is_empty() {
            println!(
                "{} installing base packages: {}",
                "→".cyan(),
                base_pkgs.join(", ").bold()
            );
            install_packages_with_fallback(rootfs, &base_pkgs, distro_str, &HashMap::new())?;
            installed_packages.extend(base_pkgs);
        }

        for step in &t.steps {
            if !step_applies(step, distro_str, answers) {
                continue;
            }

            println!("{} {}…", "→".cyan(), step.name.bold());

            if !step.tools.is_empty() {
                let pkgs = translate_list(&step.tools, distro_str);
                println!("  {} packages: {}", "↳".dimmed(), pkgs.join(", ").dimmed());
                install_packages_with_fallback(rootfs, &pkgs, distro_str, &t.runtime.env)?;
                installed_packages.extend(pkgs);
            }

            for cmd in &step.commands {
                run_command(rootfs, cmd, &t.runtime.env)?;
            }
        }

        for cmd in &t.runtime.commands {
            run_command(rootfs, cmd, &t.runtime.env)?;
        }
    }

    if !extra_packages.is_empty() {
        let translated = translate_list(extra_packages, distro_str);
        println!(
            "{} installing extra packages: {}",
            "→".cyan(),
            translated.join(", ").bold()
        );
        install_packages_with_fallback(rootfs, &translated, distro_str, &HashMap::new())?;
        installed_packages.extend(translated);
    }

    if let Some(shell) = chosen_shell {
        if shell != "bash" {
            let shell_pkg = translate_list(&[shell.to_string()], distro_str);
            println!("{} installing shell: {}…", "→".cyan(), shell.bold());
            install_packages_with_fallback(rootfs, &shell_pkg, distro_str, &HashMap::new())?;
            installed_packages.extend(shell_pkg);
        }

        let shell_path = format!("/bin/{shell}");
        let shell_bin = format!("/usr/{shell}");
        // Set the default shell for root.
        let chsh_root = format!(
            "chsh -s {shell_path} root 2>/dev/null || chsh -s {shell_bin} root 2>/dev/null || true"
        );
        let _ = run_command(rootfs, &chsh_root, &HashMap::new());

        // Also set the default shell for the mapped container user.
        if host_user.uid != 0 {
            let chsh_user = format!(
                "chsh -s {shell_path} {username} 2>/dev/null || chsh -s {shell_bin} {username} 2>/dev/null || true",
                username = host_user.username
            );
            let _ = run_command(rootfs, &chsh_user, &HashMap::new());
        }
    }

    // Create /workspace and assign it to the mapped user so the host user
    // can write project files without permission errors.
    run_command(rootfs, "mkdir -p /workspace", &HashMap::new())?;

    let final_repo = if let Some(url) = repo {
        // Derive the target directory name from the URL, stripping a trailing
        // ".git" suffix if present (e.g. "https://github.com/foo/bar.git" → "bar").
        let repo_name = url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("repo")
            .trim_end_matches(".git");
        let clone_dest = format!("/workspace/{repo_name}");
        println!(
            "{} cloning {} into {}…",
            "→".cyan(),
            url.bold(),
            clone_dest.dimmed()
        );
        run_command(
            rootfs,
            &format!("git clone {url} {clone_dest}"),
            &HashMap::new(),
        )
        .map_err(|_| DxonError::GitCloneFailed(url.to_string()))?;
        Some(url.to_string())
    } else {
        None
    };

    // Recursively chown /workspace to the mapped UID/GID (covers both the
    // empty directory case and any files placed there by git clone above).
    let chown_cmd = format!("chown -R {}:{} /workspace", host_user.uid, host_user.gid);
    let _ = run_command(rootfs, &chown_cmd, &HashMap::new());

    Ok((installed_packages, final_repo))
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
