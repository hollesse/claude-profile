//! Integration tests against a temporary HOME – the real ~/.claude is never touched.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

const FAKE_CLAUDE: &str = r#"#!/bin/sh
echo "CONFIG=$CLAUDE_CONFIG_DIR"
echo "PROFILE=$CLAUDE_PROFILE"
echo "VERTEX=${CLAUDE_CODE_USE_VERTEX-unset}"
echo "OTEL=${OTEL_EXPORTER_OTLP_ENDPOINT-unset}"
echo "KEEP=${KEEP_ME-unset}"
echo "PLUGINS=${CLAUDE_CODE_PLUGIN_DIRS-unset}"
echo "ARGS=$*"
"#;

struct Env {
    _tmp: TempDir,
    home: PathBuf,
    bin: PathBuf,
}

impl Env {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let bin = tmp.path().join("bin");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&bin).unwrap();
        let claude = bin.join("claude");
        fs::write(&claude, FAKE_CLAUDE).unwrap();
        fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
        Env { _tmp: tmp, home, bin }
    }

    fn cmd(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_claude-profile"));
        c.env_clear()
            .env("HOME", &self.home)
            .env("PATH", format!("{}:/usr/bin:/bin", self.bin.display()))
            .current_dir(&self.home)
            .stdin(Stdio::null());
        c
    }

    fn run(&self, args: &[&str]) -> Output {
        self.cmd().args(args).output().unwrap()
    }

    fn ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "{args:?} failed:\n{}\n{}",
            stdout(&out),
            stderr(&out)
        );
        stdout(&out)
    }

    /// Two profiles, symlink to vertex.
    fn with_two_profiles() -> Self {
        let e = Env::new();
        e.ok(&["add", "subscription", "--forbid-env-template", "subscription", "--yes"]);
        e.ok(&["add", "vertex", "--link-target", "vertex", "--yes"]);
        e
    }

    fn link_target(&self) -> PathBuf {
        fs::read_link(self.home.join(".claude")).unwrap()
    }
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap()
}

#[test]
fn run_without_profile_and_tty_exits_2() {
    let e = Env::with_two_profiles();
    let out = e.run(&["run", "--"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("no profile selected"));
    assert!(stdout(&out).is_empty(), "claude must not have been started");
}

#[test]
fn run_resume_uses_the_profile_the_session_ran_under() {
    let e = Env::with_two_profiles();
    let id = "8d2182af-6c52-4815-a30c-e9e15c75738e";
    let other = "11111111-2222-3333-4444-555555555555";
    let projects = e.home.join(".claude-vertex/projects/-tmp-x");
    fs::create_dir_all(&projects).unwrap();
    fs::write(projects.join(format!("{id}.jsonl")), "{}\n").unwrap();

    // Without CLAUDE_PROFILE and without a terminal: the session determines the profile.
    for args in [vec!["--resume", id], vec!["-r", id, "--fork-session"]] {
        let mut full = vec!["run", "--"];
        full.extend(args);
        let out = e.run(&full);
        assert!(out.status.success(), "{}", stderr(&out));
        assert!(stdout(&out).contains("PROFILE=vertex"), "{}", stdout(&out));
        assert!(stdout(&out).contains(&format!("ARGS={}", full[2..].join(" "))));
    }
    let out = e.run(&["run", "--", &format!("--resume={id}")]);
    assert!(stdout(&out).contains("PROFILE=vertex"));

    // A differing CLAUDE_PROFILE is skipped for this session – with a note.
    let out = e.cmd().env("CLAUDE_PROFILE", "subscription").args(["run", "--", "--resume", id]).output().unwrap();
    assert!(stdout(&out).contains("PROFILE=vertex"));
    assert!(stderr(&out).contains("ran under `vertex` – ignoring CLAUDE_PROFILE=subscription for it"), "{}", stderr(&out));

    // Unknown session, session name or no value: as before.
    for args in [vec!["--resume", other], vec!["--resume", "titel"], vec!["--resume"]] {
        let mut full = vec!["run", "--"];
        full.extend(args);
        let out = e.run(&full);
        assert_eq!(out.status.code(), Some(2), "{full:?}");
        assert!(stderr(&out).contains("no profile selected"));
    }
    let out = e.cmd().env("CLAUDE_PROFILE", "subscription").args(["run", "--", "--resume", other]).output().unwrap();
    assert!(stdout(&out).contains("PROFILE=subscription"));
}

#[test]
fn run_without_any_profile_exits_2() {
    let e = Env::new();
    let out = e.run(&["run", "--"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn unknown_profile_in_env_is_an_error() {
    let e = Env::with_two_profiles();
    let out = e.cmd().env("CLAUDE_PROFILE", "foo").args(["run", "--"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("unknown profile `foo`"));
    assert!(stdout(&out).is_empty());
}

#[test]
fn subscription_scrubs_forbidden_env_and_sets_config_dir() {
    let e = Env::with_two_profiles();
    let out = e
        .cmd()
        .env("CLAUDE_PROFILE", "subscription")
        .env("CLAUDE_CODE_USE_VERTEX", "1")
        .env("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317")
        .env("KEEP_ME", "yes")
        .args(["run", "--", "-p", "hello world"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let so = stdout(&out);
    assert!(so.contains(&format!("CONFIG={}", e.home.join(".claude-subscription").display())));
    assert!(so.contains("PROFILE=subscription"));
    assert!(so.contains("VERTEX=unset"));
    assert!(so.contains("OTEL=unset"));
    assert!(so.contains("KEEP=yes"));
    assert!(so.contains("ARGS=-p hello world"));
    let se = stderr(&out);
    assert!(se.contains("claude-profile: subscription (~/.claude-subscription)"));
    assert!(se.contains("CLAUDE_CODE_USE_VERTEX") && se.contains("OTEL_EXPORTER_OTLP_ENDPOINT"));
    assert!(!se.contains("localhost"), "values must not be printed");
}

#[test]
fn vertex_keeps_env_and_sets_config_dir_even_for_link_target() {
    let e = Env::with_two_profiles();
    let out = e
        .cmd()
        .env("CLAUDE_PROFILE", "vertex")
        .env("CLAUDE_CODE_USE_VERTEX", "1")
        .env("CLAUDE_PROFILE_QUIET", "1")
        .args(["run", "--"])
        .output()
        .unwrap();
    let so = stdout(&out);
    assert!(so.contains(&format!("CONFIG={}", e.home.join(".claude-vertex").display())));
    assert!(so.contains("VERTEX=1"));
    assert!(stderr(&out).is_empty());
}

#[test]
fn badge_mod_is_installed_and_passed_to_claude() {
    let e = Env::with_two_profiles();
    let mod_dir = e.home.join(".config/claude-profiles/mods/profile-badge");
    assert!(!mod_dir.exists());

    let out = e
        .cmd()
        .env("CLAUDE_CODE_PLUGIN_DIRS", "/some/other/mod")
        .args(["start", "vertex"])
        .output()
        .unwrap();
    assert!(stdout(&out).contains(&format!("PLUGINS={}:/some/other/mod", mod_dir.display())));
    assert!(read(&mod_dir.join("hooks/register.tsx")).contains("CLAUDE_PROFILE"));
    assert!(mod_dir.join("hooks/hooks.json").is_file());
    assert!(mod_dir.join(".claude-plugin/plugin.json").is_file());

    // Outdated files are replaced on the next start.
    fs::write(mod_dir.join("hooks/register.tsx"), "old").unwrap();
    e.ok(&["start", "vertex"]);
    assert!(read(&mod_dir.join("hooks/register.tsx")).contains("CLAUDE_PROFILE"));

    // Can be turned off.
    let out = e
        .cmd()
        .env("CLAUDE_PROFILE_NO_BADGE", "1")
        .args(["start", "vertex"])
        .output()
        .unwrap();
    assert!(stdout(&out).contains("PLUGINS=unset"));

    // Desktop gets the profile name and the mod folder; --dry-run writes nothing.
    fs::remove_dir_all(&mod_dir).unwrap();
    let so = e.ok(&["desktop", "vertex", "--dry-run", "--no-app"]);
    assert!(so.contains("--env CLAUDE_PROFILE=vertex"));
    assert!(so.contains(&format!("CLAUDE_CODE_PLUGIN_DIRS={}", mod_dir.display())));
    assert!(!mod_dir.exists());
}

fn registry(e: &Env) -> serde_json::Value {
    serde_json::from_str(&read(&e.home.join(".config/claude-profiles/profiles.json"))).unwrap()
}

#[test]
fn badge_colors_are_suggested_distinct_and_settable() {
    // Without a value: the first free suggestion per profile.
    let e = Env::with_two_profiles();
    let reg = registry(&e);
    assert_eq!(reg["profiles"]["subscription"]["badge"]["bg"], "#4c9aff");
    assert_eq!(reg["profiles"]["subscription"]["badge"]["fg"], "#000000");
    assert_eq!(reg["profiles"]["vertex"]["badge"]["bg"], "#f5a623");

    // Custom values on creation; the text color follows from the contrast.
    let so = e.ok(&["add", "dunkel", "--badge-bg", "123", "--yes"]);
    assert!(so.contains("badge: text #ffffff on #112233"));
    let reg = registry(&e);
    assert_eq!(reg["profiles"]["dunkel"]["badge"]["bg"], "#112233");
    assert_eq!(reg["profiles"]["dunkel"]["badge"]["fg"], "#ffffff");

    // Invalid values abort before anything is created.
    let out = e.run(&["add", "kaputt", "--badge-bg", "blau", "--yes"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(!e.home.join(".claude-kaputt").exists());
    assert_eq!(e.run(&["add", "kaputt", "--badge-fg", "#000"]).status.code(), Some(2));

    // Change afterwards.
    e.ok(&["badge", "vertex", "--bg", "#E5484D", "--fg", "#FFFFFF"]);
    let reg = registry(&e);
    assert_eq!(reg["profiles"]["vertex"]["badge"]["bg"], "#e5484d");
    assert_eq!(reg["profiles"]["vertex"]["badge"]["fg"], "#ffffff");
    assert_eq!(reg["profiles"]["vertex"]["dir"], "~/.claude-vertex", "the rest is kept");

    // Warnings: poor contrast, same color as another profile.
    let out = e.run(&["badge", "vertex", "--bg", "#4c9aff", "--fg", "#5c9aff"]);
    assert!(out.status.success());
    let se = stderr(&out);
    assert!(se.contains("low contrast"), "{se}");
    assert!(se.contains("`subscription` has the same background color"), "{se}");

    // Without values the picker is needed – without a terminal that is an abort.
    assert_eq!(e.run(&["badge", "vertex"]).status.code(), Some(2));
    assert_eq!(e.run(&["badge", "nope", "--bg", "#fff"]).status.code(), Some(2));
}

#[test]
fn profile_without_stored_color_counts_with_its_name_color() {
    // Like a profile from an older version: no badge entry. The mod draws `acme` in
    // pink (#ff7eb6); a new profile must not get this color suggested.
    let e = Env::new();
    e.ok(&["add", "acme", "--yes"]);
    let path = e.home.join(".config/claude-profiles/profiles.json");
    let mut reg = registry(&e);
    reg["profiles"]["acme"].as_object_mut().unwrap().remove("badge");
    fs::write(&path, reg.to_string()).unwrap();

    let out = e.run(&["add", "zwei", "--badge-bg", "#ff7eb6", "--dry-run"]);
    assert!(stderr(&out).contains("`acme` has the same background color"));
    e.ok(&["start", "acme"]); // a registry without badge stays readable
}

#[test]
fn start_ignores_claude_profile_env() {
    let e = Env::with_two_profiles();
    let out = e
        .cmd()
        .env("CLAUDE_PROFILE", "vertex")
        .args(["start", "subscription", "--", "--version"])
        .output()
        .unwrap();
    let so = stdout(&out);
    assert!(so.contains("PROFILE=subscription"));
    assert!(so.contains("ARGS=--version"));

    let out = e.run(&["start", "nope"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn add_rejects_invalid_name() {
    let e = Env::new();
    for bad in ["a b", "x/y", "ä", "a_b"] {
        let out = e.run(&["add", bad, "--yes"]);
        assert_eq!(out.status.code(), Some(2), "{bad}");
    }
    assert!(!e.home.join(".config").exists());
}

#[test]
fn add_adopts_existing_dir_without_data_loss() {
    let e = Env::new();
    let old = e.home.join(".claude");
    fs::create_dir_all(old.join("projects")).unwrap();
    fs::write(old.join("settings.json"), "{\"a\":1}").unwrap();
    fs::write(old.join("projects/x.jsonl"), "data").unwrap();
    fs::write(e.home.join(".claude.json"), "{\"state\":true}").unwrap();

    // Without a decision there is no guessing.
    let out = e.run(&["add", "vertex", "--yes"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(old.join("settings.json").exists());

    // dry-run changes nothing.
    let so = e.ok(&["add", "vertex", "--adopt", "--dry-run"]);
    assert!(so.contains("rename: ~/.claude → ~/.claude-vertex"));
    assert!(fs::symlink_metadata(&old).unwrap().is_dir());
    assert!(!e.home.join(".claude-vertex").exists());
    assert!(!e.home.join(".config").exists());

    // Without --yes and without a terminal: abort, nothing changed.
    let out = e.run(&["add", "vertex", "--adopt"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(fs::symlink_metadata(&old).unwrap().is_dir());

    e.ok(&["add", "vertex", "--adopt", "--yes"]);
    let dir = e.home.join(".claude-vertex");
    assert_eq!(read(&dir.join("settings.json")), "{\"a\":1}");
    assert_eq!(read(&dir.join("projects/x.jsonl")), "data");
    assert_eq!(read(&dir.join(".claude.json")), "{\"state\":true}");
    // ~/.claude.json points as a symlink to the file in the profile.
    assert_eq!(fs::read_link(e.home.join(".claude.json")).unwrap(), dir.join(".claude.json"));
    assert_eq!(read(&e.home.join(".claude.json")), "{\"state\":true}");
    assert!(fs::symlink_metadata(&old).unwrap().file_type().is_symlink());
    assert_eq!(e.link_target(), dir);
    // Writing through the symlink lands in the profile (like the setup tool does).
    fs::write(old.join("settings.json"), "{\"a\":2}").unwrap();
    assert_eq!(read(&dir.join("settings.json")), "{\"a\":2}");

    let reg: serde_json::Value =
        serde_json::from_str(&read(&e.home.join(".config/claude-profiles/profiles.json"))).unwrap();
    assert_eq!(reg["link_target"], "vertex");
    assert_eq!(reg["profiles"]["vertex"]["dir"], "~/.claude-vertex");
    assert_eq!(reg["claude_bin"], e.bin.join("claude").display().to_string());

    // Idempotent.
    let so = e.ok(&["add", "vertex", "--yes"]);
    assert!(so.contains("nothing to do"));
}

#[test]
fn adopt_rewrites_paths_that_point_at_the_default_dir() {
    let e = Env::new();
    let old = e.home.join(".claude");
    let h = e.home.display().to_string();
    fs::create_dir_all(old.join("plugins")).unwrap();
    fs::create_dir_all(old.join("hooks")).unwrap();
    fs::write(old.join("hooks/sound.sh"), "#!/bin/sh\n").unwrap();
    let settings = r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"~/.claude/hooks/sound.sh Stop"}]}],"X":[{"hooks":[{"type":"command","command":"$HOME/.claude/hooks/sound.sh X"}]}]},"note":"~/.claude.json and ~/.claude-other/ stay"}"#;
    fs::write(old.join("settings.json"), settings).unwrap();
    let plugins = format!(r#"{{"plugins":{{"a":[{{"installPath":"{h}/.claude/plugins/cache/a/1.0"}}]}}}}"#);
    fs::write(old.join("plugins/installed_plugins.json"), &plugins).unwrap();

    let so = e.ok(&["add", "acme", "--adopt", "--dry-run"]);
    assert!(so.contains("adjust paths: ~/.claude/ → ~/.claude-acme/ in settings.json (2), plugins/installed_plugins.json (1)"), "{so}");
    assert_eq!(read(&old.join("settings.json")), settings, "dry-run changes nothing");

    e.ok(&["add", "acme", "--adopt", "--yes"]);
    let dir = e.home.join(".claude-acme");
    let new = read(&dir.join("settings.json"));
    assert!(new.contains(r#""command":"~/.claude-acme/hooks/sound.sh Stop""#));
    assert!(new.contains(r#""command":"$HOME/.claude-acme/hooks/sound.sh X""#));
    assert!(new.contains("~/.claude.json and ~/.claude-other/ stay"));
    assert_eq!(
        read(&dir.join("plugins/installed_plugins.json")),
        format!(r#"{{"plugins":{{"a":[{{"installPath":"{h}/.claude-acme/plugins/cache/a/1.0"}}]}}}}"#)
    );
    // Backups with the old content are next to them.
    let backup = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|d| d.ok())
        .find(|d| d.file_name().to_string_lossy().starts_with("settings.json.claude-profile-backup-"))
        .unwrap();
    assert_eq!(read(&backup.path()), settings);
    assert!(stderr(&e.run(&["list"])).is_empty());

    // Paths created later: list warns, fix-paths repairs.
    fs::write(dir.join("settings.json"), settings).unwrap();
    assert!(stderr(&e.run(&["list"])).contains("contains 2 paths to ~/.claude/"));
    e.ok(&["fix-paths", "acme", "--dry-run"]);
    assert_eq!(read(&dir.join("settings.json")), settings);
    assert_eq!(e.run(&["fix-paths", "acme"]).status.code(), Some(2), "no change without --yes");
    e.ok(&["fix-paths", "acme", "--yes"]);
    assert!(read(&dir.join("settings.json")).contains("~/.claude-acme/hooks/sound.sh Stop"));
    assert!(e.ok(&["fix-paths", "acme", "--yes"]).contains("nothing to do"));
    assert_eq!(e.run(&["fix-paths", "nope"]).status.code(), Some(2));
}

#[test]
fn add_new_leaves_real_dir_untouched() {
    let e = Env::new();
    let old = e.home.join(".claude");
    fs::create_dir_all(&old).unwrap();
    fs::write(old.join("settings.json"), "{}").unwrap();
    e.ok(&["add", "subscription", "--new", "--yes"]);
    assert!(fs::symlink_metadata(&old).unwrap().is_dir());
    assert!(old.join("settings.json").exists());
    assert!(e.home.join(".claude-subscription").is_dir());
    let out = e.run(&["list"]);
    assert!(stderr(&out).contains("real directory"));
    // default leaves a real directory untouched.
    let out = e.run(&["default", "subscription", "--yes"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(fs::symlink_metadata(&old).unwrap().is_dir());
}

#[test]
fn add_refuses_to_overwrite_existing_target() {
    let e = Env::new();
    fs::create_dir_all(e.home.join(".claude")).unwrap();
    fs::create_dir_all(e.home.join(".claude-vertex")).unwrap();
    let out = e.run(&["add", "vertex", "--adopt", "--yes"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(fs::symlink_metadata(e.home.join(".claude")).unwrap().is_dir());
}

#[test]
fn list_shows_markers_and_warnings() {
    let e = Env::with_two_profiles();
    fs::write(
        e.home.join(".envrc"),
        "# BEGIN claude-profile\nexport CLAUDE_PROFILE=vertex\n# END claude-profile\n",
    )
    .unwrap();
    let out = e.cmd().env("CLAUDE_PROFILE", "subscription").arg("list").output().unwrap();
    let so = stdout(&out);
    let line = |n: &str| so.lines().find(|l| l.starts_with(n)).unwrap().to_string();
    assert!(line("subscription").contains("← active"));
    assert!(!line("subscription").contains("← symlink"));
    assert!(line("vertex").contains("← symlink"));
    assert!(line("vertex").contains("← direnv"));
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));

    assert_eq!(e.ok(&["list", "--names"]), "subscription\nvertex\n");

    // Broken symlink.
    fs::rename(e.home.join(".claude-vertex"), e.home.join("moved")).unwrap();
    let out = e.run(&["list"]);
    assert!(out.status.success());
    assert!(stderr(&out).contains("broken"));
    assert!(stdout(&out).contains("missing"));
}

#[test]
fn default_switches_symlink() {
    let e = Env::with_two_profiles();
    assert_eq!(e.link_target(), e.home.join(".claude-vertex"));
    let out = e.run(&["default", "subscription"]);
    assert_eq!(out.status.code(), Some(2), "no change without --yes and a terminal");
    assert_eq!(e.link_target(), e.home.join(".claude-vertex"));
    e.ok(&["default", "subscription", "--yes"]);
    assert_eq!(e.link_target(), e.home.join(".claude-subscription"));
    assert!(e.home.join(".claude-vertex").is_dir());
    let out = e.run(&["default", "nope", "--yes"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn claude_json_follows_the_symlink() {
    let e = Env::with_two_profiles();
    let json = e.home.join(".claude.json");
    let sub = e.home.join(".claude-subscription/.claude.json");
    let vertex = e.home.join(".claude-vertex/.claude.json");
    assert_eq!(fs::read_link(&json).unwrap(), vertex);

    e.ok(&["default", "subscription", "--yes"]);
    assert_eq!(fs::read_link(&json).unwrap(), sub);

    // A program replaces the symlink with a real file: the profile has none yet → move.
    fs::remove_file(&json).unwrap();
    fs::write(&json, "stray-1").unwrap();
    let out = e.run(&["list"]);
    assert!(stderr(&out).contains("~/.claude.json is not a symlink"));
    let out = e.run(&["default", "subscription"]);
    assert_eq!(out.status.code(), Some(2), "no change without --yes");
    assert_eq!(read(&json), "stray-1");
    e.ok(&["default", "subscription", "--yes"]);
    assert_eq!(fs::read_link(&json).unwrap(), sub);
    assert_eq!(read(&sub), "stray-1");

    // The profile already has one: the real file is renamed, not deleted or overwritten.
    fs::remove_file(&json).unwrap();
    fs::write(&json, "stray-2").unwrap();
    e.ok(&["default", "subscription", "--yes"]);
    assert_eq!(fs::read_link(&json).unwrap(), sub);
    assert_eq!(read(&sub), "stray-1");
    let backups: Vec<_> = fs::read_dir(&e.home)
        .unwrap()
        .filter_map(|d| d.ok())
        .filter(|d| d.file_name().to_string_lossy().starts_with(".claude.json.claude-profile-backup-"))
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(read(&backups[0].path()), "stray-2");

    let so = e.ok(&["default", "subscription", "--yes"]);
    assert!(so.contains("already point"));
    assert!(stderr(&e.run(&["list"])).is_empty());
}

#[test]
fn set_writes_and_removes_envrc_block() {
    let e = Env::with_two_profiles();
    let proj = e.home.join("proj");
    fs::create_dir_all(&proj).unwrap();
    let envrc = proj.join(".envrc");
    fs::write(&envrc, "export FOO=1\n").unwrap();
    let set = |args: &[&str]| e.cmd().current_dir(&proj).args(args).output().unwrap();

    assert!(set(&["set", "vertex"]).status.success());
    assert!(set(&["set", "vertex"]).status.success());
    assert_eq!(
        read(&envrc),
        "export FOO=1\n# BEGIN claude-profile\nexport CLAUDE_PROFILE=vertex\n# END claude-profile\n"
    );
    assert!(set(&["set", "subscription"]).status.success());
    assert_eq!(read(&envrc).matches("CLAUDE_PROFILE=").count(), 1);
    assert!(read(&envrc).contains("CLAUDE_PROFILE=subscription"));

    assert_eq!(set(&["set", "nope"]).status.code(), Some(2));
    assert!(set(&["set", "--unset"]).status.success());
    assert_eq!(read(&envrc), "export FOO=1\n");
}

#[test]
fn replace_claude_command_is_idempotent_and_removable() {
    let e = Env::with_two_profiles();
    let rc = e.home.join(".zshrc");
    let orig = "export PATH=$HOME/bin:$PATH\nalias ll='ls -l'\n";
    fs::write(&rc, orig).unwrap();

    let out = e.run(&["replace-claude-command", "--shell", "zsh"]);
    assert_eq!(out.status.code(), Some(2), "no change without --yes");
    assert_eq!(read(&rc), orig);

    e.ok(&["replace-claude-command", "--shell", "zsh", "--yes"]);
    let once = read(&rc);
    assert!(once.starts_with(orig));
    assert!(once.contains("claude() { command claude-profile run -- \"$@\"; }"));
    e.ok(&["replace-claude-command", "--shell", "zsh", "--yes"]);
    assert_eq!(read(&rc), once);
    assert_eq!(once.matches("# BEGIN claude-profile").count(), 1);

    let backups: Vec<_> = fs::read_dir(&e.home)
        .unwrap()
        .filter_map(|d| d.ok())
        .filter(|d| d.file_name().to_string_lossy().starts_with(".zshrc.claude-profile-backup-"))
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(read(&backups[0].path()), orig);

    e.ok(&["replace-claude-command", "--shell", "zsh", "--remove", "--yes"]);
    assert_eq!(read(&rc), orig);

    // Shell from $SHELL; bash writes ~/.bashrc even if the file does not exist yet.
    let out = e
        .cmd()
        .env("SHELL", "/bin/bash")
        .args(["replace-claude-command", "--yes"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(read(&e.home.join(".bashrc")).contains("claude-profile run"));
}

#[test]
fn shell_function_works_in_bash_and_zsh() {
    let e = Env::with_two_profiles();
    let self_dir = Path::new(env!("CARGO_BIN_EXE_claude-profile")).parent().unwrap();
    for (shell, rc) in [("bash", ".bashrc"), ("zsh", ".zshrc")] {
        e.ok(&["replace-claude-command", "--shell", shell, "--yes"]);
        let script = format!(". \"$HOME/{rc}\"; claude --flag 'a b'");
        let out = Command::new(format!("/bin/{shell}"))
            .env_clear()
            .env("HOME", &e.home)
            .env(
                "PATH",
                format!("{}:{}:/usr/bin:/bin", self_dir.display(), e.bin.display()),
            )
            .env("CLAUDE_PROFILE", "subscription")
            .args(["-c", &script])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(out.status.success(), "{shell}: {}", stderr(&out));
        let so = stdout(&out);
        assert!(so.contains("PROFILE=subscription"), "{shell}: {so}");
        assert!(so.contains("ARGS=--flag a b"), "{shell}: {so}");
    }
}

#[test]
fn desktop_dry_run_prints_open_command() {
    let e = Env::with_two_profiles();
    // --no-app: otherwise a starter app in the real /Applications would count.
    let so = e.ok(&["desktop", "subscription", "--dry-run", "--no-app"]);
    assert!(so.starts_with("/usr/bin/open -n -a Claude --env CLAUDE_CONFIG_DIR="));
    assert!(so.contains(&format!("{}", e.home.join(".claude-subscription").display())));
    assert!(so.contains("Library/Application Support/Claude-subscription"));
    // Without a name and without a terminal nothing is started.
    let out = e.run(&["desktop", "--dry-run"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stdout(&out).is_empty());
}

#[test]
fn desktop_prefers_an_installed_starter_app() {
    let e = Env::with_two_profiles();
    let apps = e.home.join("Applications");
    fs::create_dir_all(&apps).unwrap();
    e.ok(&["desktop-app", "vertex", "--dir", apps.to_str().unwrap()]);
    let app = apps.join("Claude (Vertex).app");
    // Installed: the starter app is opened; it starts the instance with the profile icon.
    let so = e.ok(&["desktop", "vertex", "--dry-run"]);
    assert_eq!(so.trim_end(), format!("/usr/bin/open '{}'", app.display()));
    // --no-app: Claude.app directly, as without a starter app.
    let so = e.ok(&["desktop", "vertex", "--dry-run", "--no-app"]);
    assert!(so.starts_with("/usr/bin/open -n -a Claude --env CLAUDE_CONFIG_DIR="), "{so}");
}

// macOS only: the launcher's window is a Mach-O binary that build.rs compiles with swiftc.
#[cfg(target_os = "macos")]
#[test]
fn desktop_app_and_launcher_create_bundles() {
    // Always with --dir: the default is the real /Applications and off limits for tests.
    let e = Env::with_two_profiles();
    let apps = e.home.join("apps");
    fs::create_dir_all(&apps).unwrap();
    let dir = apps.to_str().unwrap();

    e.ok(&["desktop-app", "vertex", "--dir", dir]);
    e.ok(&["desktop-app", "vertex", "--dir", dir]); // again: overwrites its own bundle
    let app = apps.join("Claude (Vertex).app");
    let exe = app.join("Contents/MacOS/launcher");
    assert!(read(&app.join("Contents/Info.plist")).contains("local.claude-profile.vertex"));
    assert!(read(&exe).contains("desktop vertex --exec"));
    // The profile app appears in the Dock, the launcher does not.
    let background = |app: &str| {
        let plist = read(&apps.join(app).join("Contents/Info.plist"));
        plist.split("<key>LSUIElement</key>").nth(1).unwrap().trim_start().starts_with("<true/>")
    };
    assert!(!background("Claude (Vertex).app"));
    assert!(fs::metadata(&exe).unwrap().permissions().mode() & 0o111 != 0);

    e.ok(&["launcher", "--dir", dir]);
    let launcher = apps.join("Claude Profiles.app/Contents/MacOS/launcher");
    // The launcher is a small app of its own with a window: a Mach-O binary that appears in the
    // Dock and knows the claude-profile binary from the Info.plist.
    assert!(fs::read(&launcher).unwrap().starts_with(&[0xcf, 0xfa, 0xed, 0xfe]));
    assert!(fs::metadata(&launcher).unwrap().permissions().mode() & 0o111 != 0);
    assert!(!background("Claude Profiles.app"));
    let plist = read(&apps.join("Claude Profiles.app/Contents/Info.plist"));
    assert!(plist.contains("<key>ClaudeProfileBin</key>"));
    assert!(plist.contains(env!("CARGO_BIN_EXE_claude-profile")) || plist.contains("claude-profile</string>"));

    // What the window shows: every profile with its icon and – if installed – its starter app.
    let data: serde_json::Value = serde_json::from_str(&e.ok(&["launcher-data", "--dir", dir])).unwrap();
    let profiles = data["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[0]["id"], "subscription");
    assert!(profiles[0]["app"].is_null(), "not installed yet");
    assert_eq!(profiles[1]["id"], "vertex");
    assert!(profiles[1]["app"].as_str().unwrap().ends_with("apps/Claude (Vertex).app"));
    for p in profiles {
        let icon = fs::read(p["icon"].as_str().unwrap()).unwrap();
        assert!(icon.starts_with(b"\x89PNG"));
    }
    // Both bundles come with an icon of their own; the profile app's depends on the profile.
    let icon = |app: &str| {
        let contents = apps.join(app).join("Contents");
        assert!(read(&contents.join("Info.plist")).contains("<key>CFBundleIconFile</key>"));
        let data = fs::read(contents.join("Resources/AppIcon.icns")).unwrap();
        assert!(data.starts_with(b"icns"));
        data
    };
    assert_ne!(icon("Claude Profiles.app"), icon("Claude (Vertex).app"));
    e.ok(&["desktop-app", "subscription", "--dir", dir]);
    assert_ne!(icon("Claude (Subscription).app"), icon("Claude (Vertex).app"));

    assert_eq!(e.run(&["desktop-app", "nope", "--dir", dir]).status.code(), Some(2));

    // Foreign bundles are not overwritten.
    let foreign = apps.join("Claude (Subscription).app/Contents");
    fs::create_dir_all(&foreign).unwrap();
    fs::write(foreign.join("Info.plist"), "<plist/>").unwrap();
    assert_eq!(
        e.run(&["desktop-app", "subscription", "--dir", dir]).status.code(),
        Some(1)
    );
    assert_eq!(read(&foreign.join("Info.plist")), "<plist/>");
}

#[test]
fn profile_ids_are_lowercase_and_the_spelling_is_only_shown() {
    let e = Env::with_two_profiles();

    // Created as ACME: lowercase internally, the display name is stored next to it.
    let so = e.ok(&["add", "ACME", "--yes"]);
    assert!(so.contains("Profile `ACME` set up (~/.claude-acme)"), "{so}");
    let reg = registry(&e);
    assert_eq!(reg["profiles"]["acme"]["name"], "ACME");
    assert_eq!(reg["profiles"]["acme"]["dir"], "~/.claude-acme");
    assert!(reg["profiles"].get("ACME").is_none());
    assert!(reg["profiles"]["vertex"].get("name").is_none(), "lowercase: no entry");

    // The same profile in a different capitalization does not create a second one.
    let so = e.ok(&["add", "acme", "--yes"]);
    assert!(so.contains("Profile `ACME` is already set up"), "{so}");
    assert_eq!(registry(&e)["profiles"].as_object().unwrap().len(), 3);

    // Any capitalization starts the profile. Claude Code gets the ID, ACME is shown.
    for spelling in ["acme", "ACME", "Acme", "aCMe"] {
        let out = e.run(&["start", spelling]);
        assert!(stdout(&out).contains("PROFILE=acme"), "{spelling}");
        assert!(stderr(&out).contains("claude-profile: ACME (~/.claude-acme)"), "{spelling}");
        let out = e.cmd().env("CLAUDE_PROFILE", spelling).args(["run", "--"]).output().unwrap();
        assert!(stdout(&out).contains("PROFILE=acme"), "{spelling}");
    }

    // List: the table shows the display name, --names the IDs.
    assert_eq!(e.ok(&["list", "--names"]), "acme\nsubscription\nvertex\n");
    let out = e.cmd().env("CLAUDE_PROFILE", "Acme").arg("list").output().unwrap();
    assert!(stdout(&out).lines().any(|l| l.starts_with("ACME") && l.contains("← active")));
    assert!(!stderr(&out).contains("not a known profile"), "{}", stderr(&out));

    // The other commands also take any capitalization and store the ID.
    e.ok(&["badge", "Acme", "--bg", "#c678dd"]);
    assert_eq!(registry(&e)["profiles"]["acme"]["badge"]["bg"], "#c678dd");
    e.ok(&["default", "ACME", "--yes"]);
    assert_eq!(registry(&e)["link_target"], "acme");
    e.ok(&["set", "ACME"]);
    assert!(read(&e.home.join(".envrc")).contains("export CLAUDE_PROFILE=acme"));

    // rename only changes the display.
    e.ok(&["rename", "VERTEX", "Vertex"]);
    let reg = registry(&e);
    assert_eq!(reg["profiles"]["vertex"]["name"], "Vertex");
    assert_eq!(reg["profiles"]["vertex"]["dir"], "~/.claude-vertex");
    e.ok(&["rename", "Vertex", "vertex"]);
    assert!(registry(&e)["profiles"]["vertex"].get("name").is_none());
    assert_eq!(e.run(&["rename", "vertex", "vertex-ai"]).status.code(), Some(2));
    assert_eq!(e.run(&["rename", "nope", "NOPE"]).status.code(), Some(2));
    assert_eq!(e.run(&["add", "With Space", "--yes"]).status.code(), Some(2));

    // The starter app carries the display name and is renamed on change, not duplicated.
    let apps = e.home.join("apps");
    fs::create_dir_all(&apps).unwrap();
    let dir = apps.to_str().unwrap();
    e.ok(&["desktop-app", "subscription", "--dir", dir]);
    e.ok(&["rename", "subscription", "SUBSCRIPTION"]);
    e.ok(&["desktop-app", "subscription", "--dir", dir]);
    let names: Vec<_> = fs::read_dir(&apps)
        .unwrap()
        .map(|d| d.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, ["Claude (SUBSCRIPTION).app"]);
    let app = apps.join("Claude (SUBSCRIPTION).app/Contents");
    assert!(read(&app.join("Info.plist")).contains("<string>local.claude-profile.subscription</string>"));
    assert!(read(&app.join("MacOS/launcher")).contains("desktop subscription"));
    let dry = e.ok(&["desktop", "Subscription", "--dry-run", "--no-app"]);
    assert!(dry.contains("CLAUDE_PROFILE=subscription"), "{dry}");
    assert!(dry.contains("Application Support/Claude-subscription"), "{dry}");
}

#[test]
fn registry_with_uppercase_keys_is_read_as_lowercase_ids() {
    // Entered by hand or from an older version: the key becomes the display name.
    let e = Env::new();
    e.ok(&["add", "acme", "--yes"]);
    let path = e.home.join(".config/claude-profiles/profiles.json");
    let mut reg = registry(&e);
    let profile = reg["profiles"].as_object_mut().unwrap().remove("acme").unwrap();
    reg["profiles"]["ACME"] = profile;
    reg["link_target"] = "ACME".into();
    fs::write(&path, reg.to_string()).unwrap();

    let out = e.run(&["start", "acme"]);
    assert!(stdout(&out).contains("PROFILE=acme"));
    assert!(stderr(&out).contains("claude-profile: ACME ("));
    // Afterwards the registry is in the new form – the badge mod reads it directly.
    let reg = registry(&e);
    assert_eq!(reg["profiles"]["acme"]["name"], "ACME");
    assert!(reg["profiles"].get("ACME").is_none());
    assert_eq!(reg["link_target"], "acme");
}
