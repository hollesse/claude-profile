//! `claude-profile add` – creates a profile. Never deletes anything (PRD section 7, S5).

use crate::error::usage;
use crate::link::{self, JsonAction, LinkState};
use crate::registry::{profile_id, spelling, validate_name, Badge, Profile, Registry};
use crate::rewrite::{self, Finding};
use crate::{color, launch, paths, prompt};
use anyhow::{bail, Context, Result};
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;

pub const SUBSCRIPTION_FORBID_ENV: &[&str] = &[
    "CLAUDE_CODE_USE_VERTEX",
    "ANTHROPIC_VERTEX_*",
    "ANTHROPIC_BASE_URL",
    "GOOGLE_CLOUD_*",
    "OTEL_*",
];

#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
pub enum Template {
    /// Removes Vertex, gateway and telemetry variables
    Subscription,
    /// Remove no variables
    None,
}

pub struct AddOpts {
    pub name: Option<String>,
    pub adopt: bool,
    pub fresh: bool,
    pub link_target: Option<String>,
    pub template: Option<Template>,
    pub badge_bg: Option<String>,
    pub badge_fg: Option<String>,
    pub dry_run: bool,
    pub yes: bool,
}

enum Step {
    Rename { from: PathBuf, to: PathBuf },
    Json { action: JsonAction, dir: PathBuf },
    FixPaths { findings: Vec<Finding>, dir: PathBuf },
    Mkdir(PathBuf),
    Link { profile: String, dir: PathBuf },
    Register { forbid_env: Vec<String>, badge: Badge },
}

impl Step {
    fn describe(&self) -> String {
        match self {
            Step::Rename { from, to } => format!(
                "rename: {} → {}",
                paths::tilde(from),
                paths::tilde(to)
            ),
            Step::Json { action, dir } => link::describe_json(action, dir),
            Step::FixPaths { findings, dir } => rewrite::describe(findings, dir),
            Step::Mkdir(dir) => format!("create: {}", paths::tilde(dir)),
            Step::Link { profile, dir } => format!(
                "symlink: ~/.claude → {} (profile {profile})",
                paths::tilde(dir)
            ),
            Step::Register { forbid_env, badge } => format!(
                "register in profiles.json (forbid_env: {}; badge: {})",
                if forbid_env.is_empty() {
                    "empty".to_string()
                } else {
                    forbid_env.join(", ")
                },
                color::summary(badge)
            ),
        }
    }

    fn needs_confirmation(&self) -> bool {
        matches!(
            self,
            Step::Rename { .. } | Step::Json { .. } | Step::Link { .. }
        )
    }
}

pub fn run(mut opts: AddOpts) -> Result<()> {
    let mut reg = Registry::load()?;

    let name = match &opts.name {
        Some(n) => n.clone(),
        None => prompt::input("Profile name (a-z, A-Z, 0-9, -)")?,
    };
    validate_name(&name)?;
    // Internally the lowercase ID counts; the capitalization of the input is display only.
    let shown = if reg.contains(&name) {
        reg.shown(&name).to_string()
    } else {
        name.clone()
    };
    let name = profile_id(&name);
    opts.link_target = opts.link_target.map(|t| profile_id(&t));

    let already = reg.profiles.contains_key(&name);
    let dir = match reg.profiles.get(&name) {
        Some(p) => p.dir_path()?,
        None => paths::profile_dir(&name)?,
    };
    let link_path = paths::claude_link()?;
    let state = link::state()?;
    if state == LinkState::Other {
        bail!("~/.claude is neither a directory nor a symlink – please resolve manually");
    }

    let mut steps = Vec::new();
    let mut notes = Vec::new();

    // Step 2/3: adopt the existing ~/.claude or create a new directory.
    let can_adopt = state == LinkState::RealDir && !already;
    if opts.adopt && !can_adopt {
        return Err(usage(if already {
            format!("--adopt not possible: profile `{name}` already exists")
        } else {
            "--adopt not possible: ~/.claude is not a real directory".to_string()
        }));
    }
    let adopt = if !can_adopt || opts.fresh {
        false
    } else if opts.adopt {
        true
    } else if prompt::interactive() {
        let items = vec![
            format!(
                "Adopt the existing ~/.claude (rename to {})",
                paths::tilde(&dir)
            ),
            "Create a new empty profile (~/.claude stays unchanged)".to_string(),
        ];
        prompt::select("~/.claude is a real directory", &items, 0)? == 0
    } else {
        return Err(usage(
            "~/.claude is a real directory: please pass --adopt (adopt it) or --new (new empty profile)",
        ));
    };

    if adopt {
        if fs::symlink_metadata(&dir).is_ok() {
            bail!(
                "{} already exists – adopting would overwrite it",
                paths::tilde(&dir)
            );
        }
        steps.push(Step::Rename {
            from: link_path.clone(),
            to: dir.clone(),
        });
        // The directory's references to itself (hooks, plugins) would break
        // as soon as ~/.claude points to another profile.
        let findings = rewrite::scan(&link_path, &dir)?;
        if !findings.is_empty() {
            steps.push(Step::FixPaths {
                findings,
                dir: dir.clone(),
            });
        }
    } else {
        match fs::symlink_metadata(&dir) {
            Ok(_) if dir.is_dir() => {}
            Ok(_) => bail!("{} exists but is not a directory", paths::tilde(&dir)),
            Err(_) => steps.push(Step::Mkdir(dir.clone())),
        }
    }

    // Step 4: where does the symlink ~/.claude point?
    let mut candidates = reg.names();
    if !already {
        candidates.push(name.clone());
    }
    let state_after = if adopt { LinkState::Missing } else { state.clone() };
    let current = link::linked_profile(&reg, &state);
    let chosen: Option<String> = match &state_after {
        LinkState::RealDir => {
            if opts.link_target.is_some() {
                return Err(usage(
                    "--link-target not possible: ~/.claude is a real directory (adopt it with --adopt first)",
                ));
            }
            notes.push(
                "~/.claude stays a real directory – it points to no profile.".to_string(),
            );
            None
        }
        LinkState::Missing => Some(match &opts.link_target {
            Some(t) => t.clone(),
            None if candidates.len() == 1 => candidates[0].clone(),
            // Adopting without a terminal: ~/.claude keeps pointing at the same data as before.
            None if adopt && !prompt::interactive() => name.clone(),
            None if prompt::interactive() => {
                let default = candidates.iter().position(|c| *c == name).unwrap_or(0);
                candidates[prompt::select(
                    "Which profile should the symlink ~/.claude point to?",
                    &candidates,
                    default,
                )?]
                .clone()
            }
            None => {
                return Err(usage(format!(
                    "symlink ~/.claude is missing: please pass --link-target <profile> ({})",
                    candidates.join(", ")
                )))
            }
        }),
        LinkState::Symlink { .. } => match &opts.link_target {
            Some(t) => Some(t.clone()),
            None if prompt::interactive() && !opts.yes => {
                let keep = match &current {
                    Some(c) => format!("leave unchanged (points to {c})"),
                    None => "leave unchanged".to_string(),
                };
                let mut items = vec![keep];
                items.extend(candidates.iter().cloned());
                let idx = prompt::select("Symlink ~/.claude", &items, 0)?;
                (idx > 0).then(|| candidates[idx - 1].clone())
            }
            None => None,
        },
        LinkState::Other => unreachable!(),
    };
    if let Some(target) = &chosen {
        if !candidates.contains(target) {
            return Err(usage(format!(
                "unknown profile `{target}` for the symlink ({})",
                candidates.join(", ")
            )));
        }
        let target_dir = if *target == name {
            dir.clone()
        } else {
            reg.get(target)?.dir_path()?
        };
        if current.as_ref() != Some(target) {
            steps.push(Step::Link {
                profile: target.clone(),
                dir: target_dir.clone(),
            });
        }
        // ~/.claude.json follows the symlink ~/.claude. When adopting, the profile still lives
        // under ~/.claude at planning time.
        let has_json = if adopt && *target == name {
            link_path.join(".claude.json").exists()
        } else {
            target_dir.join(".claude.json").exists()
        };
        let action = link::json_action(&target_dir, has_json)?;
        if action != JsonAction::Nothing {
            steps.push(Step::Json {
                action,
                dir: target_dir,
            });
        }
    } else if let (Some(cur), LinkState::Symlink { .. }) = (&current, &state_after) {
        // The symlink stays, but ~/.claude.json may not match it.
        let cur_dir = reg.get(cur)?.dir_path()?;
        if link::json_action(&cur_dir, cur_dir.join(".claude.json").exists())? != JsonAction::Nothing {
            notes.push(format!(
                "~/.claude.json does not point to the profile `{cur}` – `claude-profile default {cur}` repairs that."
            ));
        }
    }

    // Step 5: forbid_env template.
    if !already {
        let template = match opts.template {
            Some(t) => t,
            None if prompt::interactive() && !opts.yes => {
                let items = vec![
                    format!("Subscription ({})", SUBSCRIPTION_FORBID_ENV.join(", ")),
                    "None (e.g. for Vertex)".to_string(),
                ];
                let default = if name == "subscription" { 0 } else { 1 };
                match prompt::select(
                    "Which environment variables should be removed on start?",
                    &items,
                    default,
                )? {
                    0 => Template::Subscription,
                    _ => Template::None,
                }
            }
            None => Template::None,
        };
        let forbid_env = match template {
            Template::Subscription => SUBSCRIPTION_FORBID_ENV.iter().map(|s| s.to_string()).collect(),
            Template::None => Vec::new(),
        };
        // Step 6: colors of the profile badge.
        let badge = color::pick(
            &reg,
            &name,
            &shown,
            opts.badge_bg.as_deref(),
            opts.badge_fg.as_deref(),
            prompt::interactive() && !opts.yes,
        )?;
        steps.push(Step::Register { forbid_env, badge });
    }

    if steps.is_empty() {
        println!("Profile `{shown}` is already set up – nothing to do.");
        for n in &notes {
            println!("Note: {n}");
        }
        return Ok(());
    }

    println!("Plan for profile `{shown}`:");
    for (i, s) in steps.iter().enumerate() {
        println!("  {}. {}", i + 1, s.describe());
    }
    for n in &notes {
        println!("Note: {n}");
    }
    if opts.dry_run {
        println!("(dry-run – nothing changed)");
        return Ok(());
    }
    if steps.iter().any(Step::needs_confirmation) && !prompt::confirm("Run the plan?", opts.yes)? {
        println!("Aborted – nothing changed.");
        return Ok(());
    }

    for step in &steps {
        match step {
            Step::FixPaths { dir, .. } => {
                rewrite::apply(dir)?;
            }
            Step::Json { action, dir } => {
                if let Some(backup) = link::apply_json(action, dir)? {
                    println!("Backup: {}", paths::tilde(&backup));
                }
            }
            Step::Rename { from, to } => {
                fs::rename(from, to).with_context(|| {
                    format!(
                        "cannot move {} to {}",
                        paths::tilde(from),
                        paths::tilde(to)
                    )
                })?;
            }
            Step::Mkdir(d) => {
                fs::DirBuilder::new()
                    .mode(0o700)
                    .create(d)
                    .with_context(|| format!("cannot create {}", paths::tilde(d)))?;
            }
            Step::Link { profile, dir } => {
                link::set(dir)?;
                reg.link_target = Some(profile.clone());
            }
            Step::Register { forbid_env, badge } => {
                reg.profiles.insert(
                    name.clone(),
                    Profile {
                        name: spelling(&shown),
                        dir: paths::tilde(&dir),
                        forbid_env: forbid_env.clone(),
                        desktop_user_data_dir: None,
                        badge: Some(badge.clone()),
                    },
                );
            }
        }
    }
    if reg.claude_bin.is_none() {
        match launch::find_claude_in_path() {
            Some(bin) => reg.claude_bin = Some(bin.display().to_string()),
            None => eprintln!(
                "claude-profile: warning: `claude` not found in PATH – `claude_bin` stays empty"
            ),
        }
    }
    reg.save()?;

    println!("Profile `{shown}` set up ({}).", paths::tilde(&dir));
    if adopt {
        println!(
            "Note: on macOS the login's keychain entry may be tied to the config directory – \
             if Claude is no longer logged in after the rename, run `claude-profile start {name}` and `/login` once."
        );
    }
    Ok(())
}
