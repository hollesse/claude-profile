use crate::error::usage;
use crate::paths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub link_target: Option<String>,
    #[serde(default)]
    pub claude_bin: Option<String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Display name of the profile, e.g. `ACME` for the profile `acme`.
    /// If missing, the lowercase key is shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub dir: String,
    #[serde(default)]
    pub forbid_env: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_user_data_dir: Option<String>,
    /// Colors of the profile badge; if missing, the mod picks a color from the name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub badge: Option<Badge>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Badge {
    /// Background color, `#rrggbb`
    pub bg: String,
    /// Text color, `#rrggbb`
    pub fg: String,
}

impl Profile {
    /// The name as it is shown. `id` is the profile's key in the registry.
    pub fn shown<'a>(&'a self, id: &'a str) -> &'a str {
        self.name.as_deref().unwrap_or(id)
    }

    pub fn dir_path(&self) -> Result<PathBuf> {
        paths::expand(&self.dir)
    }
}

impl Registry {
    pub fn load() -> Result<Self> {
        let path = paths::registry_path()?;
        match fs::read_to_string(&path) {
            Ok(s) => {
                let mut reg: Self = serde_json::from_str(&s)
                    .with_context(|| format!("{} is not valid JSON", paths::tilde(&path)))?;
                if reg.lowercase_ids() {
                    // Save right away: the badge mod reads the file directly and expects
                    // lowercase keys. If that fails, it only applies to this run.
                    let _ = reg.save();
                }
                Ok(reg)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("cannot read {}", paths::tilde(&path))),
        }
    }

    /// Internally a profile is always lowercase. A key with uppercase letters (entered by hand
    /// or from an older version) becomes the display name.
    /// Returns whether anything changed.
    fn lowercase_ids(&mut self) -> bool {
        let mixed: Vec<String> = self
            .profiles
            .keys()
            .filter(|k| k.bytes().any(|b| b.is_ascii_uppercase()))
            .cloned()
            .collect();
        for key in &mixed {
            let mut profile = self.profiles.remove(key).expect("key comes from the map");
            profile.name.get_or_insert(key.clone());
            self.profiles.entry(profile_id(key)).or_insert(profile);
        }
        let link = self.link_target.as_deref().map(profile_id);
        let changed = !mixed.is_empty() || link != self.link_target;
        self.link_target = link;
        changed
    }

    /// Writes atomically (temporary file + rename).
    pub fn save(&self) -> Result<()> {
        let path = paths::registry_path()?;
        let dir = path.parent().expect("registry path has a parent directory");
        fs::create_dir_all(dir)
            .with_context(|| format!("cannot create {}", paths::tilde(dir)))?;
        let tmp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        fs::write(&tmp, json)
            .with_context(|| format!("cannot write {}", paths::tilde(&tmp)))?;
        fs::rename(&tmp, &path)
            .with_context(|| format!("cannot write {}", paths::tilde(&path)))?;
        Ok(())
    }

    /// Whether the profile exists; the capitalization of the input does not matter.
    pub fn contains(&self, name: &str) -> bool {
        self.profiles.contains_key(&profile_id(name))
    }

    /// The internal lowercase ID for an input in any capitalization.
    /// An unknown profile is an error.
    pub fn id(&self, name: &str) -> Result<String> {
        self.get(name)?;
        Ok(profile_id(name))
    }

    /// The name as it is shown; for an unknown profile the input itself.
    pub fn shown<'a>(&'a self, name: &'a str) -> &'a str {
        match self.profiles.get_key_value(&profile_id(name)) {
            Some((id, profile)) => profile.shown(id),
            None => name,
        }
    }

    /// Finds the profile regardless of capitalization.
    pub fn get(&self, name: &str) -> Result<&Profile> {
        self.profiles.get(&profile_id(name)).ok_or_else(|| {
            let known = if self.profiles.is_empty() {
                "no profiles yet – run `claude-profile add` first".to_string()
            } else {
                format!(
                    "known: {}",
                    self.profiles
                        .iter()
                        .map(|(id, p)| p.shown(id))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            usage(format!("unknown profile `{name}` ({known})"))
        })
    }

    /// The internal IDs of all profiles, alphabetically.
    pub fn names(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }
}

/// Internal ID of a profile: the name in lowercase. It is used in `profiles.json`,
/// in `CLAUDE_PROFILE`, in `.envrc` and in directory names.
pub fn profile_id(name: &str) -> String {
    name.to_ascii_lowercase()
}

/// The display name stored next to the ID – only if it differs from it.
pub fn spelling(name: &str) -> Option<String> {
    (name != profile_id(name)).then(|| name.to_string())
}

pub fn validate_name(name: &str) -> Result<()> {
    let ok = !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(usage(format!(
            "invalid profile name `{name}` (allowed: a-z, A-Z, 0-9, -)"
        )))
    }
}
