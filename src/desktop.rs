//! Claude Desktop per profile (macOS): `desktop`, `desktop-app`, `launcher`.

use crate::registry::Registry;
use crate::{badge, color, icon, launch, paths, prompt};
use anyhow::{bail, Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const BUNDLE_ID_PREFIX: &str = "local.claude-profile.";

fn require_macos() -> Result<()> {
    if cfg!(target_os = "macos") {
        Ok(())
    } else {
        bail!("Claude Desktop is only supported on macOS")
    }
}

fn user_data_dir(reg: &Registry, name: &str) -> Result<PathBuf> {
    match &reg.get(name)?.desktop_user_data_dir {
        Some(d) => paths::expand(d),
        None => Ok(paths::home()?.join(format!("Library/Application Support/Claude-{name}"))),
    }
}

/// Arguments for `/usr/bin/open`.
fn open_args(reg: &Registry, name: &str) -> Result<Vec<String>> {
    let dir = reg.get(name)?.dir_path()?;
    if !dir.is_dir() {
        bail!("profile directory {} for `{name}` is missing", paths::tilde(&dir));
    }
    let mut args: Vec<String> = vec![
        "-n".into(),
        "-a".into(),
        "Claude".into(),
        "--env".into(),
        format!("CLAUDE_CONFIG_DIR={}", dir.display()),
        "--env".into(),
        format!("CLAUDE_PROFILE={name}"),
    ];
    if badge::enabled() {
        args.push("--env".into());
        args.push(format!(
            "{}={}",
            badge::PLUGIN_DIRS_ENV,
            badge::plugin_dirs(&badge::dir()?).to_string_lossy()
        ));
    }
    args.push("--args".into());
    args.push(format!("--user-data-dir={}", user_data_dir(reg, name)?.display()));
    Ok(args)
}

fn applescript_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// AppleScript for the profile picker of “Claude Profiles.app”.
fn choose_script(names: &[&str]) -> String {
    let items: Vec<String> = names.iter().map(|n| applescript_string(n)).collect();
    format!(
        "choose from list {{{}}} with title \"Claude Profiles\" with prompt \"Profile for Claude Desktop:\"",
        items.join(", ")
    )
}

fn osascript(script: &str) -> Result<String> {
    let out = Command::new("/usr/bin/osascript")
        .args(["-e", script])
        .output()
        .context("cannot start /usr/bin/osascript")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string())
}

/// Asks for the profile in a dialog. `None`: cancelled or no profiles.
fn choose(reg: &Registry) -> Result<Option<String>> {
    if reg.profiles.is_empty() {
        osascript(
            "display alert \"Claude Profiles\" message \"No profiles yet (claude-profile add).\"",
        )?;
        return Ok(None);
    }
    let items: Vec<&str> = reg.profiles.iter().map(|(id, p)| p.shown(id)).collect();
    // The dialog shows the display names; on cancel osascript returns `false`.
    let choice = osascript(&choose_script(&items))?;
    Ok(reg
        .profiles
        .iter()
        .find(|(id, p)| p.shown(id) == choice)
        .map(|(id, _)| id.clone()))
}

/// The Claude Desktop binary, for starting without `open`.
fn claude_desktop_bin() -> Result<PathBuf> {
    let rel = "Applications/Claude.app/Contents/MacOS/Claude";
    [PathBuf::from("/").join(rel), paths::home()?.join(rel)]
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| anyhow::anyhow!("Claude.app not found (neither in /Applications nor in ~/Applications)"))
}

/// Replaces the process with Claude Desktop. Started from a starter app, the instance
/// then belongs to that app for the Dock and carries its icon. Only returns on error.
fn exec_in_place(reg: &Registry, name: &str, dry_run: bool) -> Result<()> {
    use std::os::unix::process::CommandExt;
    let profile = reg.get(name)?;
    let dir = profile.dir_path()?;
    if !dir.is_dir() {
        bail!("profile directory {} for `{name}` is missing", paths::tilde(&dir));
    }
    let bin = claude_desktop_bin()?;
    let data_dir = format!("--user-data-dir={}", user_data_dir(reg, name)?.display());
    if dry_run {
        println!("{} {}", sh_quote(&bin.display().to_string()), sh_quote(&data_dir));
        return Ok(());
    }
    let mut cmd = Command::new(&bin);
    cmd.arg(data_dir);
    // Unlike `open`, exec passes on its own environment – so forbid_env applies here too.
    for (key, _) in std::env::vars_os() {
        let Some(key) = key.to_str() else { continue };
        if profile.forbid_env.iter().any(|p| launch::glob_match(p, key)) {
            cmd.env_remove(key);
        }
    }
    cmd.env("CLAUDE_CONFIG_DIR", &dir);
    cmd.env("CLAUDE_PROFILE", name);
    if badge::enabled() {
        match badge::ensure() {
            Ok(mod_dir) => {
                cmd.env(badge::PLUGIN_DIRS_ENV, badge::plugin_dirs(&mod_dir));
            }
            Err(e) => eprintln!("claude-profile: warning: profile badge not available: {e:#}"),
        }
    }
    let err = cmd.exec();
    Err(anyhow::anyhow!(err).context(format!("cannot start {}", bin.display())))
}

/// Folders in which a starter app counts as installed.
fn app_dirs() -> Result<Vec<PathBuf>> {
    Ok(vec![PathBuf::from("/Applications"), paths::home()?.join("Applications")])
}

/// The installed starter app of the profile `id`, if there is one.
fn starter_app(id: &str) -> Result<Option<PathBuf>> {
    let bundle_id = format!("{BUNDLE_ID_PREFIX}{id}");
    Ok(app_dirs()?.iter().find_map(|d| find_bundle(d, &bundle_id)))
}

/// Asks whether the missing starter app should be installed, and then creates it.
/// `None`: open without a starter app.
fn offer_starter_app(reg: &Registry, name: &str) -> Result<Option<PathBuf>> {
    let items = vec![
        "Install the starter app in /Applications and open with it (profile icon in the Dock)".to_string(),
        "Open without a starter app (regular Claude icon)".to_string(),
    ];
    let msg = format!("No starter app is installed for `{}`", reg.shown(name));
    if prompt::select(&msg, &items, 0)? != 0 {
        return Ok(None);
    }
    let app = create_app(reg, name, None)?;
    eprintln!("claude-profile: created {}", paths::tilde(&app));
    Ok(Some(app))
}

pub fn desktop(name: Option<&str>, dry_run: bool, dialog: bool, in_place: bool, no_app: bool) -> Result<()> {
    let reg = Registry::load()?;
    let name = if dialog {
        require_macos()?;
        match choose(&reg)? {
            Some(name) => name,
            None => return Ok(()),
        }
    } else {
        launch::resolve(&reg, name, true)?
    };
    if in_place {
        require_macos()?;
        return exec_in_place(&reg, &name, dry_run);
    }
    // Without a starter app: Claude.app directly, as a separate instance with the regular Claude icon.
    let plain = open_args(&reg, &name)?;
    // An installed starter app takes precedence: it starts the same instance with the profile icon.
    let mut app = if no_app { None } else { starter_app(&name)? };
    if dry_run {
        let args = match &app {
            Some(app) => vec![app.display().to_string()],
            None => plain,
        };
        println!("/usr/bin/open {}", args.iter().map(|a| sh_quote(a)).collect::<Vec<_>>().join(" "));
        return Ok(());
    }
    require_macos()?;
    // Without a terminal (script, the launcher's picker dialog) there is no question.
    if app.is_none() && !no_app && !dialog && prompt::interactive() {
        app = offer_starter_app(&reg, &name)?;
    }
    if badge::enabled() {
        if let Err(e) = badge::ensure() {
            eprintln!("claude-profile: warning: profile badge not available: {e:#}");
        }
    }
    let args = match &app {
        Some(app) => vec![app.display().to_string()],
        None => plain,
    };
    let status = Command::new("/usr/bin/open")
        .args(&args)
        .status()
        .context("cannot start /usr/bin/open")?;
    if !status.success() {
        bail!("Claude Desktop could not be started (is Claude.app installed?)");
    }
    eprintln!(
        "claude-profile: Desktop started for {}",
        color::label(&name, reg.get(&name)?, 0)
    );
    Ok(())
}

fn sh_quote(s: &str) -> String {
    if !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_=/.:,@".contains(&b))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The path of this binary as it was started, made absolute but with symlinks kept: the
/// apps store it, and a package manager's stable link (Homebrew: `…/bin/claude-profile`)
/// survives an upgrade, while the versioned path behind it does not.
fn self_path() -> Result<PathBuf> {
    std::env::current_exe()
        .and_then(std::path::absolute)
        .context("cannot determine the path of the own binary")
}

/// The bundle in the folder `dir` that carries the bundle ID `bundle_id`.
fn find_bundle(dir: &Path, bundle_id: &str) -> Option<PathBuf> {
    let id_entry = format!("<string>{bundle_id}</string>");
    fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|app| {
        fs::read_to_string(app.join("Contents/Info.plist"))
            .map(|s| s.contains(&id_entry))
            .unwrap_or(false)
    })
}

/// What makes up a claude-profile bundle.
struct Bundle<'a> {
    app_name: &'a str,
    bundle_id: &'a str,
    /// Content of `Contents/MacOS/launcher`: a shell script or a binary.
    executable: &'a [u8],
    icon: &'a [u8],
    /// false: a pure helper without a Dock icon.
    in_dock: bool,
    /// Additional entries for the Info.plist, as ready-made XML.
    extra_plist: &'a str,
}

/// Writes a minimal app bundle.
/// Only overwrites bundles created by claude-profile.
fn write_bundle(dir: Option<&Path>, bundle: &Bundle) -> Result<PathBuf> {
    let Bundle {
        app_name,
        bundle_id,
        executable,
        icon,
        in_dock,
        extra_plist,
    } = *bundle;
    let dir = match dir {
        Some(d) => d.to_path_buf(),
        None => PathBuf::from("/Applications"),
    };
    let app = dir.join(format!("{app_name}.app"));
    let plist = app.join("Contents/Info.plist");
    if fs::symlink_metadata(&app).is_ok() {
        let ours = fs::read_to_string(&plist)
            .map(|s| s.contains(BUNDLE_ID_PREFIX))
            .unwrap_or(false);
        if !ours {
            bail!(
                "{} exists and was not created by claude-profile – not overwriting it",
                paths::tilde(&app)
            );
        }
    }
    // If the app had a different name so far (new display name of the profile), it is renamed
    // instead of duplicated.
    if let Some(old) = find_bundle(&dir, bundle_id) {
        if old.file_name() != app.file_name() {
            // Fails if both apps already exist – then the old one stays.
            let _ = fs::rename(&old, &app);
        }
    }
    let macos = app.join("Contents/MacOS");
    fs::create_dir_all(&macos).with_context(|| {
        format!(
            "cannot create {} (without write access to the folder: --dir ~/Applications)",
            paths::tilde(&macos)
        )
    })?;
    let name = xml_escape(app_name);
    let res = app.join("Contents/Resources");
    fs::create_dir_all(&res)?;
    let icon_file = res.join("AppIcon.icns");
    fs::write(&icon_file, icon)
        .with_context(|| format!("cannot write {}", paths::tilde(&icon_file)))?;
    fs::write(
        &plist,
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key>
	<string>{name}</string>
	<key>CFBundleDisplayName</key>
	<string>{name}</string>
	<key>CFBundleIdentifier</key>
	<string>{bundle_id}</string>
	<key>CFBundleExecutable</key>
	<string>launcher</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>{version}</string>
	<key>CFBundleVersion</key>
	<string>{version}</string>
	<key>LSUIElement</key>
	<{background}/>
	<key>NSHighResolutionCapable</key>
	<true/>
	<key>CFBundleIconFile</key>
	<string>AppIcon</string>
{extra_plist}</dict>
</plist>
"#,
            version = env!("CARGO_PKG_VERSION"),
            background = !in_dock
        ),
    )
    .with_context(|| format!("cannot write {}", paths::tilde(&plist)))?;
    let exe = macos.join("launcher");
    fs::write(&exe, executable).with_context(|| format!("cannot write {}", paths::tilde(&exe)))?;
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o755))?;
    // New modification time on the bundle so Finder and Dock re-read a changed icon.
    if let Ok(f) = fs::File::open(&app) {
        let _ = f.set_modified(std::time::SystemTime::now());
    }
    Ok(app)
}

fn title(name: &str) -> String {
    let mut c = name.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn script_header(bin: &Path) -> String {
    format!("#!/bin/sh\n# Created by claude-profile\nBIN={}\n", sh_quote(&bin.display().to_string()))
}

pub fn desktop_app(name: &str, dir: Option<&Path>) -> Result<()> {
    let reg = Registry::load()?;
    let name = &reg.id(name)?;
    let app = create_app(&reg, name, dir)?;
    println!(
        "Created {} – starts Claude Desktop with profile `{}`.",
        paths::tilde(&app),
        reg.shown(name)
    );
    Ok(())
}

/// Creates the starter app of the profile `name` (ID) and returns its path.
fn create_app(reg: &Registry, name: &str, dir: Option<&Path>) -> Result<PathBuf> {
    let profile = reg.get(name)?;
    let icon = icon::profile(profile.shown(name), &color::effective(name, profile))?;
    // Without a display name of its own, the name with a capital first letter.
    let shown = match &profile.name {
        Some(n) => n.clone(),
        None => title(name),
    };
    // --exec: the instance runs as a process of this app and shows its icon in the Dock.
    let script = format!("{}exec \"$BIN\" desktop {name} --exec\n", script_header(&self_path()?));
    write_bundle(
        dir,
        &Bundle {
            app_name: &format!("Claude ({shown})"),
            bundle_id: &format!("{BUNDLE_ID_PREFIX}{name}"),
            executable: script.as_bytes(),
            icon: &icon,
            in_dock: true,
            extra_plist: "",
        },
    )
}

/// The launcher's window, compiled by build.rs from launcher/Launcher.swift.
/// Empty if no `swiftc` was available at build time.
const LAUNCHER_UI: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/launcher-ui"));

pub fn launcher(dir: Option<&Path>) -> Result<()> {
    let bin = self_path()?;
    let icon = icon::launcher()?;
    let bundle_id = format!("{BUNDLE_ID_PREFIX}launcher");
    let app = if LAUNCHER_UI.is_empty() {
        // Without the window: picker dialog, then `desktop` starts the instance.
        let script = format!("{}exec \"$BIN\" desktop --dialog\n", script_header(&bin));
        write_bundle(
            dir,
            &Bundle {
                app_name: "Claude Profiles",
                bundle_id: &bundle_id,
                executable: script.as_bytes(),
                icon: &icon,
                in_dock: false,
                extra_plist: "",
            },
        )?
    } else {
        // The window asks this binary for the profiles and has it create starter apps.
        let extra = format!(
            "\t<key>ClaudeProfileBin</key>\n\t<string>{}</string>\n\t<key>LSMinimumSystemVersion</key>\n\t<string>13.0</string>\n",
            xml_escape(&bin.display().to_string())
        );
        write_bundle(
            dir,
            &Bundle {
                app_name: "Claude Profiles",
                bundle_id: &bundle_id,
                executable: LAUNCHER_UI,
                icon: &icon,
                in_dock: true,
                extra_plist: &extra,
            },
        )?
    };
    println!(
        "Created {} – shows the profiles and opens Claude Desktop for the chosen one.",
        paths::tilde(&app)
    );
    Ok(())
}

#[derive(serde::Serialize)]
struct LauncherProfile {
    /// Lowercase ID
    id: String,
    /// Display name
    name: String,
    /// PNG of the profile icon
    icon: PathBuf,
    /// Installed starter app, if there is one
    app: Option<PathBuf>,
}

/// `claude-profile launcher-data --dir <folder>`: what the launcher's window shows, as JSON.
/// A starter app counts as installed if it lives in `dir` or one of the Applications folders.
pub fn launcher_data(dir: &Path) -> Result<()> {
    let reg = Registry::load()?;
    let icons = paths::cache_dir()?.join("launcher-icons");
    fs::create_dir_all(&icons)
        .with_context(|| format!("cannot create {}", paths::tilde(&icons)))?;
    let mut app_dirs = vec![dir.to_path_buf()];
    app_dirs.extend(self::app_dirs()?);
    let mut profiles = Vec::new();
    for (id, profile) in &reg.profiles {
        let icon = icons.join(format!("{id}.png"));
        let png = icon::profile_png(profile.shown(id), &color::effective(id, profile), 512)?;
        fs::write(&icon, png).with_context(|| format!("cannot write {}", paths::tilde(&icon)))?;
        let bundle_id = format!("{BUNDLE_ID_PREFIX}{id}");
        profiles.push(LauncherProfile {
            id: id.clone(),
            name: profile.shown(id).to_string(),
            icon,
            app: app_dirs.iter().find_map(|d| find_bundle(d, &bundle_id)),
        });
    }
    println!("{}", serde_json::json!({ "profiles": profiles }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose_script_quotes_names() {
        assert_eq!(
            choose_script(&["ACME", "Team \"A\" \\ B"]),
            r#"choose from list {"ACME", "Team \"A\" \\ B"} with title "Claude Profiles" with prompt "Profile for Claude Desktop:""#
        );
    }
}
