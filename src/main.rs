mod add;
mod badge;
mod block;
mod color;
mod completions;
mod desktop;
mod envrc;
mod error;
mod icon;
mod launch;
mod link;
mod list;
mod paths;
mod prompt;
mod registry;
mod resume;
mod rewrite;
mod shell;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use completions::profiles;

#[derive(Parser)]
#[command(
    name = "claude-profile",
    version,
    about = "Separate Claude Code profiles (e.g. subscription / Vertex)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a profile (interactive; never deletes anything)
    Add {
        /// Profile name (a-z, A-Z, 0-9, -); shown as written, lowercase internally
        name: Option<String>,
        /// Adopt the existing ~/.claude as this profile (rename)
        #[arg(long, conflicts_with = "fresh")]
        adopt: bool,
        /// Create a new empty profile, leave ~/.claude untouched
        #[arg(long = "new")]
        fresh: bool,
        /// Profile the symlink ~/.claude should point to
        #[arg(long, value_name = "PROFILE", add = profiles())]
        link_target: Option<String>,
        /// Template for forbid_env
        #[arg(long, value_enum, value_name = "TEMPLATE")]
        forbid_env_template: Option<add::Template>,
        /// Background color of the profile badge, e.g. #4c9aff
        #[arg(long, value_name = "COLOR")]
        badge_bg: Option<String>,
        /// Text color of the profile badge [default: black or white, depending on contrast]
        #[arg(long, value_name = "COLOR", requires = "badge_bg")]
        badge_fg: Option<String>,
        /// Only show the plan
        #[arg(long)]
        dry_run: bool,
        /// Run the plan without asking
        #[arg(long, short)]
        yes: bool,
    },
    /// Change the colors of the profile badge (without options: pick from suggestions)
    Badge {
        #[arg(add = profiles())]
        name: String,
        /// Background color, e.g. #4c9aff
        #[arg(long, value_name = "COLOR")]
        bg: Option<String>,
        /// Text color [default: black or white, depending on contrast]
        #[arg(long, value_name = "COLOR", requires = "bg")]
        fg: Option<String>,
    },
    /// Change the display name of a profile, e.g. acme → ACME
    Rename {
        #[arg(add = profiles())]
        name: String,
        /// The same name in different capitalization
        #[arg(value_name = "NEW")]
        new: String,
    },
    /// Rewrite hard-coded paths to ~/.claude/ in a profile to the profile directory
    FixPaths {
        #[arg(add = profiles())]
        name: String,
        /// Only show what would change
        #[arg(long)]
        dry_run: bool,
        #[arg(long, short)]
        yes: bool,
    },
    /// List profiles
    List {
        /// Print only the IDs (lowercase)
        #[arg(long)]
        names: bool,
    },
    /// Start Claude Code with a profile (ignores CLAUDE_PROFILE)
    Start {
        #[arg(add = profiles())]
        name: String,
        /// Arguments for claude (after `--`)
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Resolve the profile (the session's profile for `--resume <id>`, CLAUDE_PROFILE or a picker) and start Claude Code
    Run {
        /// Arguments for claude (after `--`)
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Point the symlink ~/.claude at a profile
    Default {
        #[arg(add = profiles())]
        name: String,
        #[arg(long, short)]
        yes: bool,
    },
    /// Set the profile for the current folder (direnv, .envrc)
    Set {
        #[arg(required_unless_present = "unset", conflicts_with = "unset", add = profiles())]
        name: Option<String>,
        /// Remove the block from .envrc
        #[arg(long)]
        unset: bool,
        /// Run `direnv allow` without asking
        #[arg(long, short)]
        yes: bool,
    },
    /// Add the shell function `claude` to ~/.zshrc or ~/.bashrc
    ReplaceClaudeCommand {
        #[arg(long, value_enum)]
        shell: Option<shell::Shell>,
        /// Remove the block again
        #[arg(long)]
        remove: bool,
        #[arg(long, short)]
        yes: bool,
    },
    /// Start Claude Desktop as a separate instance for a profile (macOS)
    Desktop {
        #[arg(add = profiles())]
        name: Option<String>,
        /// Only print the command
        #[arg(long)]
        dry_run: bool,
        /// Choose the profile in a macOS dialog (this is how “Claude Profiles.app” calls the command)
        #[arg(long, conflicts_with = "name")]
        dialog: bool,
        /// Start Claude Desktop in this process instead of via `open` (this is how the starter apps
        /// call the command, so the instance carries their icon in the Dock)
        #[arg(long)]
        exec: bool,
        /// Do not use an installed starter app and do not offer one: start Claude.app directly
        /// as a separate instance (regular Claude icon in the Dock)
        #[arg(long, conflicts_with = "exec")]
        no_app: bool,
    },
    /// Create the app bundle “Claude (<profile>).app” (macOS)
    DesktopApp {
        #[arg(add = profiles())]
        name: String,
        /// Target folder, e.g. ~/Applications [default: /Applications]
        #[arg(long, value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
        dir: Option<std::path::PathBuf>,
    },
    /// Print the data for the window of “Claude Profiles.app” as JSON
    #[command(hide = true)]
    LauncherData {
        /// Folder the launcher lives in and creates starter apps in
        #[arg(long, value_name = "DIR")]
        dir: std::path::PathBuf,
    },
    /// Print the shell completions: commands, options and profile names
    ///
    /// zsh:  echo 'source <(claude-profile completions zsh)' >> ~/.zshrc
    /// bash: echo 'source <(claude-profile completions bash)' >> ~/.bashrc
    /// fish: claude-profile completions fish > ~/.config/fish/completions/claude-profile.fish
    #[command(verbatim_doc_comment)]
    Completions {
        #[arg(value_enum)]
        shell: completions::Shell,
    },
    /// Create “Claude Profiles.app” with a profile picker (macOS)
    Launcher {
        /// Target folder, e.g. ~/Applications [default: /Applications]
        #[arg(long, value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
        dir: Option<std::path::PathBuf>,
    },
}

fn real_main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Add {
            name,
            adopt,
            fresh,
            link_target,
            forbid_env_template,
            badge_bg,
            badge_fg,
            dry_run,
            yes,
        } => add::run(add::AddOpts {
            name,
            adopt,
            fresh,
            link_target,
            template: forbid_env_template,
            badge_bg,
            badge_fg,
            dry_run,
            yes,
        }),
        Cmd::Badge { name, bg, fg } => color::command(&name, bg.as_deref(), fg.as_deref()),
        Cmd::Rename { name, new } => list::rename(&name, &new),
        Cmd::FixPaths { name, dry_run, yes } => rewrite::command(&name, dry_run, yes),
        Cmd::List { names } => list::list(names),
        Cmd::Start { name, args } => {
            let reg = registry::Registry::load()?;
            let name = launch::resolve(&reg, Some(&name), false)?;
            launch::launch(&reg, &name, &args)
        }
        Cmd::Run { args } => {
            let reg = registry::Registry::load()?;
            // `--resume <id>`: the profile the session ran under takes precedence.
            let name = match resume::session_profile(&reg, &args)? {
                Some(name) => name,
                None => launch::resolve(&reg, None, true)?,
            };
            launch::launch(&reg, &name, &args)
        }
        Cmd::Default { name, yes } => list::default(&name, yes),
        Cmd::Set { name, unset, yes } => envrc::set(name.as_deref(), unset, yes),
        Cmd::ReplaceClaudeCommand { shell, remove, yes } => shell::run(shell, remove, yes),
        Cmd::Desktop {
            name,
            dry_run,
            dialog,
            exec,
            no_app,
        } => desktop::desktop(name.as_deref(), dry_run, dialog, exec, no_app),
        Cmd::DesktopApp { name, dir } => desktop::desktop_app(&name, dir.as_deref()),
        Cmd::LauncherData { dir } => desktop::launcher_data(&dir),
        Cmd::Launcher { dir } => desktop::launcher(dir.as_deref()),
        Cmd::Completions { shell } => completions::print(shell),
    }
}

fn main() {
    // A call from the registered shell function: print the candidates and exit.
    clap_complete::CompleteEnv::with_factory(Cli::command)
        .var(completions::VAR)
        .completer(completions::BIN)
        .complete();

    if let Err(e) = real_main() {
        eprintln!("claude-profile: error: {e:#}");
        let code = if e.downcast_ref::<error::UsageError>().is_some() {
            2
        } else {
            1
        };
        std::process::exit(code);
    }
}
