//! Colors of the profile badge: suggestions, contrast, selection.

use crate::error::usage;
use crate::registry::{Badge, Profile, Registry};
use crate::prompt;
use anyhow::Result;

/// Suggestions for the background color. The first `FALLBACK_LEN` entries are also the
/// colors the mod assigns to a profile without a stored color based on its name –
/// order and values must match `mod/profile-badge/hooks/register.tsx`.
pub const PALETTE: &[(&str, &str)] = &[
    ("Blue", "#4c9aff"),
    ("Orange", "#f5a623"),
    ("Green", "#36b37e"),
    ("Purple", "#c678dd"),
    ("Pink", "#ff7eb6"),
    ("Teal", "#00b8d9"),
    ("Red", "#e5484d"),
    ("Yellow", "#ffd60a"),
    ("Navy", "#1f3a8a"),
    ("Gray", "#8b949e"),
];
const FALLBACK_LEN: usize = 6;

/// From this contrast ratio on, text counts as readable according to WCAG AA.
const MIN_CONTRAST: f64 = 4.5;

/// Normalizes `#rgb`, `rgb`, `#rrggbb`, `RRGGBB` to the form `#rrggbb`.
pub fn normalize(input: &str) -> Result<String> {
    let hex = input.trim().trim_start_matches('#').to_ascii_lowercase();
    let valid = (hex.len() == 3 || hex.len() == 6) && hex.bytes().all(|b| b.is_ascii_hexdigit());
    if !valid {
        return Err(usage(format!(
            "invalid color value `{input}` (expected: #rrggbb or #rgb)"
        )));
    }
    let full: String = if hex.len() == 3 {
        hex.chars().flat_map(|c| [c, c]).collect()
    } else {
        hex
    };
    Ok(format!("#{full}"))
}

/// Expects the normalized form `#rrggbb`.
fn rgb(hex: &str) -> (u8, u8, u8) {
    let v = u32::from_str_radix(&hex[1..], 16).unwrap_or(0);
    ((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Relative luminance according to WCAG 2.
fn luminance(hex: &str) -> f64 {
    let lin = |c: u8| {
        let c = c as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = rgb(hex);
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// Contrast ratio according to WCAG 2 (1 to 21).
pub fn contrast(a: &str, b: &str) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Black or white – whichever has the higher contrast on `bg`.
pub fn best_fg(bg: &str) -> &'static str {
    if contrast(bg, "#000000") >= contrast(bg, "#ffffff") {
        "#000000"
    } else {
        "#ffffff"
    }
}

/// Mixes `amount` (0 to 1) of `other` into `hex`; both in the form `#rrggbb`.
pub fn mix(hex: &str, other: &str, amount: f64) -> String {
    let (a, b) = (rgb(hex), rgb(other));
    let ch = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * amount).round() as u8;
    format!("#{:02x}{:02x}{:02x}", ch(a.0, b.0), ch(a.1, b.1), ch(a.2, b.2))
}

/// Color the mod gives a profile without a stored color (same hash as in the mod).
/// `name` is the lowercase ID.
fn fallback_bg(name: &str) -> &'static str {
    let hash = name
        .bytes()
        .fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    PALETTE[hash as usize % FALLBACK_LEN].1
}

/// The colors a profile is actually shown with.
pub fn effective(name: &str, profile: &Profile) -> Badge {
    profile.badge.clone().unwrap_or_else(|| Badge {
        bg: fallback_bg(name).to_string(),
        fg: "#000000".to_string(),
    })
}

/// Badge as a preview in the terminal (24-bit colors).
pub fn swatch(badge: &Badge, label: &str) -> String {
    let (br, bg, bb) = rgb(&badge.bg);
    let (fr, fg, fb) = rgb(&badge.fg);
    format!("\x1b[48;2;{br};{bg};{bb}m\x1b[38;2;{fr};{fg};{fb}m\x1b[1m {label} \x1b[0m")
}

/// Whether status lines and pickers on stderr may be colored (terminal, no `NO_COLOR`).
pub fn enabled() -> bool {
    use std::io::IsTerminal;
    std::io::stderr().is_terminal()
        && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
        && std::env::var("TERM").map_or(true, |t| t != "dumb")
}

/// The shown profile name as a badge in the profile's colors, padded to `width` characters;
/// without color support the padded name.
pub fn label(name: &str, profile: &Profile, width: usize) -> String {
    let padded = format!("{:width$}", profile.shown(name));
    if enabled() {
        swatch(&effective(name, profile), &padded)
    } else {
        padded
    }
}

fn describe(badge: &Badge) -> String {
    format!(
        "text {} on {} (contrast {:.1}:1)",
        badge.fg,
        badge.bg,
        contrast(&badge.bg, &badge.fg)
    )
}

/// Background colors of the other profiles.
fn used_by_others(reg: &Registry, name: &str) -> Vec<(String, String)> {
    reg.profiles
        .iter()
        .filter(|(n, _)| n.as_str() != name)
        .map(|(n, p)| (n.clone(), effective(n, p).bg))
        .collect()
}

/// Suggestions with good contrast; preferably those no other profile uses yet.
fn suggestions(reg: &Registry, name: &str) -> Vec<(&'static str, Badge)> {
    let used = used_by_others(reg, name);
    let all = PALETTE.iter().map(|(label, bg)| {
        (
            *label,
            Badge {
                bg: bg.to_string(),
                fg: best_fg(bg).to_string(),
            },
        )
    });
    let free: Vec<_> = all
        .clone()
        .filter(|(_, b)| !used.iter().any(|(_, u)| *u == b.bg))
        .collect();
    if free.is_empty() {
        all.collect()
    } else {
        free
    }
}

/// Determines the badge colors for `name`:
/// explicit values → interactive picker → first free suggestion.
/// `shown` labels the badges in the preview.
pub fn pick(
    reg: &Registry,
    name: &str,
    shown: &str,
    bg: Option<&str>,
    fg: Option<&str>,
    allow_prompt: bool,
) -> Result<Badge> {
    let badge = if let Some(bg) = bg {
        let bg = normalize(bg)?;
        let fg = match fg {
            Some(fg) => normalize(fg)?,
            None => best_fg(&bg).to_string(),
        };
        Badge { bg, fg }
    } else if allow_prompt {
        let options = suggestions(reg, name);
        let mut items: Vec<String> = options
            .iter()
            .map(|(label, b)| format!("{}  {label}: {}", swatch(b, shown), describe(b)))
            .collect();
        items.push("Enter custom values …".to_string());
        let idx = prompt::select("Badge color", &items, 0)?;
        match options.get(idx) {
            Some((_, b)) => b.clone(),
            None => {
                let check = |s: &String| normalize(s).map(|_| ()).map_err(|e| e.to_string());
                let bg = normalize(&prompt::input_validated(
                    "Background color (#rrggbb)",
                    None,
                    check,
                )?)?;
                let fg = normalize(&prompt::input_validated(
                    "Text color (#rrggbb)",
                    Some(best_fg(&bg)),
                    check,
                )?)?;
                Badge { bg, fg }
            }
        }
    } else {
        suggestions(reg, name)
            .into_iter()
            .next()
            .map(|(_, b)| b)
            .expect("the palette is not empty")
    };

    let ratio = contrast(&badge.bg, &badge.fg);
    if ratio < MIN_CONTRAST {
        eprintln!(
            "claude-profile: warning: low contrast {ratio:.1}:1 between {} and {} (recommended: at least {MIN_CONTRAST}:1, e.g. text {})",
            badge.fg,
            badge.bg,
            best_fg(&badge.bg)
        );
    }
    if let Some((other, _)) = used_by_others(reg, name)
        .into_iter()
        .find(|(_, used)| *used == badge.bg)
    {
        eprintln!(
            "claude-profile: warning: profile `{}` has the same background color {}",
            reg.shown(&other),
            badge.bg
        );
    }
    Ok(badge)
}

pub fn summary(badge: &Badge) -> String {
    describe(badge)
}

/// `claude-profile badge <name> [--bg … [--fg …]]`
pub fn command(name: &str, bg: Option<&str>, fg: Option<&str>) -> Result<()> {
    let mut reg = Registry::load()?;
    let name = &reg.id(name)?;
    let shown = reg.shown(name).to_string();
    let badge = pick(&reg, name, &shown, bg, fg, true)?;
    println!("{}  {}", swatch(&badge, &shown), describe(&badge));
    reg.profiles
        .get_mut(name)
        .expect("profile was just found")
        .badge = Some(badge);
    reg.save()?;
    println!("Saved. The color applies to newly started sessions.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_hex() {
        assert_eq!(normalize("#ABC").unwrap(), "#aabbcc");
        assert_eq!(normalize("4C9AFF").unwrap(), "#4c9aff");
        assert_eq!(normalize(" #000000 ").unwrap(), "#000000");
        for bad in ["", "#12", "#12345", "#ggg", "red", "#1234567"] {
            assert!(normalize(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn contrast_matches_wcag() {
        assert!((contrast("#000000", "#ffffff") - 21.0).abs() < 0.01);
        assert!((contrast("#ffffff", "#ffffff") - 1.0).abs() < 0.01);
        assert!((contrast("#777777", "#ffffff") - 4.48).abs() < 0.01);
        assert_eq!(best_fg("#ffd60a"), "#000000");
        assert_eq!(best_fg("#1f3a8a"), "#ffffff");
    }

    #[test]
    fn every_suggestion_is_readable_and_distinct() {
        for (label, bg) in PALETTE {
            assert!(contrast(bg, best_fg(bg)) >= MIN_CONTRAST, "{label}");
        }
        let mut bgs: Vec<_> = PALETTE.iter().map(|(_, bg)| *bg).collect();
        bgs.sort();
        bgs.dedup();
        assert_eq!(bgs.len(), PALETTE.len());
    }

    /// The hash in the mod (register.tsx) yields the same values.
    #[test]
    fn fallback_matches_the_mod() {
        assert_eq!(fallback_bg("acme"), "#ff7eb6");
        assert_eq!(fallback_bg("subscription"), "#00b8d9");
        assert_eq!(fallback_bg("vertex"), "#ff7eb6");
        assert_eq!(fallback_bg("privat"), "#36b37e");
    }
}
