//! App icons for the starter apps: SVG template → PNGs → `.icns`.

use crate::color;
use crate::registry::Badge;
use anyhow::{anyhow, Context, Result};
use resvg::{tiny_skia, usvg};

const LAUNCHER_SVG: &str = include_str!("../assets/launcher-icon.svg");
const PROFILE_SVG: &str = include_str!("../assets/profile-icon.svg");

/// Entries of an `.icns` file: type code and edge length in pixels (PNG-encoded).
const ICNS_ENTRIES: &[(&[u8; 4], u32)] = &[
    (b"ic11", 32),   // 16pt @2x
    (b"ic12", 64),   // 32pt @2x
    (b"ic07", 128),  // 128pt
    (b"ic13", 256),  // 128pt @2x
    (b"ic08", 256),  // 256pt
    (b"ic14", 512),  // 256pt @2x
    (b"ic09", 512),  // 512pt
    (b"ic10", 1024), // 512pt @2x
];

/// Space for the name in the profile icon, in units of the 1024 canvas.
const NAME_WIDTH: f64 = 680.0;
const NAME_MAX_SIZE: f64 = 132.0;
const NAME_CENTER_Y: f64 = 792.0;
/// Average glyph width of the bold font relative to the font size – an estimate for the
/// case that the text cannot be measured.
const GLYPH_WIDTH: f64 = 0.7;
const FONT: &str = "Helvetica Neue, Helvetica, Arial, sans-serif";

/// Icon of “Claude Profiles.app”.
pub fn launcher() -> Result<Vec<u8>> {
    icns(LAUNCHER_SVG, &usvg::Options::default())
}

/// Icon of a profile app: background and text in the badge colors, with the profile name below.
pub fn profile(name: &str, badge: &Badge) -> Result<Vec<u8>> {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let size = (NAME_WIDTH / text_width(name, &options)).min(NAME_MAX_SIZE);
    icns(&profile_svg(name, badge, size), &options)
}

/// Width of `text` in the bold font, relative to font size 1.
fn text_width(text: &str, options: &usvg::Options) -> f64 {
    let probe = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="4000" height="200"><text x="0" y="150" font-family="{FONT}" font-weight="bold" font-size="100" xml:space="preserve">{}</text></svg>"#,
        xml_escape(text)
    );
    let measured = usvg::Tree::from_str(&probe, options)
        .ok()
        .and_then(|tree| tree.root().abs_bounding_box())
        .map_or(0.0, |bbox| bbox.width() as f64 / 100.0);
    if measured > 0.0 {
        measured
    } else {
        GLYPH_WIDTH * text.chars().count().max(1) as f64
    }
}

/// The same icon as a single PNG with an edge length of `px` pixels, for the launcher's window.
pub fn profile_png(name: &str, badge: &Badge, px: u32) -> Result<Vec<u8>> {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let size = (NAME_WIDTH / text_width(name, &options)).min(NAME_MAX_SIZE);
    let tree = usvg::Tree::from_str(&profile_svg(name, badge, size), &options)
        .context("icon template is not valid SVG")?;
    png(&tree, px)
}

fn png(tree: &usvg::Tree, px: u32) -> Result<Vec<u8>> {
    let mut pixmap = tiny_skia::Pixmap::new(px, px)
        .ok_or_else(|| anyhow!("cannot create an image with {px} px"))?;
    let scale = px as f32 / tree.size().width();
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().context("cannot encode the icon as PNG")
}

fn profile_svg(name: &str, badge: &Badge, size: f64) -> String {
    PROFILE_SVG
        .replace("{{FONT}}", FONT)
        .replace("{{BG_TOP}}", &color::mix(&badge.bg, "#ffffff", 0.14))
        .replace("{{BG_BOTTOM}}", &color::mix(&badge.bg, "#000000", 0.12))
        .replace("{{FG}}", &badge.fg)
        .replace("{{FONT_SIZE}}", &format!("{size:.0}"))
        // Baseline such that the text is centered around NAME_CENTER_Y.
        .replace("{{TEXT_Y}}", &format!("{:.0}", NAME_CENTER_Y + 0.33 * size))
        .replace("{{NAME}}", &xml_escape(name))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn icns(svg: &str, options: &usvg::Options) -> Result<Vec<u8>> {
    let tree = usvg::Tree::from_str(svg, options).context("icon template is not valid SVG")?;

    let mut body = Vec::new();
    let mut rendered: Vec<(u32, Vec<u8>)> = Vec::new();
    for (kind, px) in ICNS_ENTRIES {
        let png = match rendered.iter().find(|(p, _)| p == px) {
            Some((_, png)) => png.clone(),
            None => {
                let png = png(&tree, *px)?;
                rendered.push((*px, png.clone()));
                png
            }
        };
        body.extend_from_slice(*kind);
        body.extend_from_slice(&(png.len() as u32 + 8).to_be_bytes());
        body.extend_from_slice(&png);
    }
    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&(body.len() as u32 + 8).to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn badge() -> Badge {
        Badge {
            bg: "#4c9aff".into(),
            fg: "#000000".into(),
        }
    }

    #[test]
    fn icns_has_header_and_all_sizes() {
        let data = launcher().unwrap();
        assert_eq!(&data[..4], b"icns");
        assert_eq!(u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize, data.len());
        let mut pos = 8;
        let mut kinds = Vec::new();
        while pos < data.len() {
            kinds.push(data[pos..pos + 4].to_vec());
            assert_eq!(&data[pos + 8..pos + 12], b"\x89PNG");
            pos += u32::from_be_bytes(data[pos + 4..pos + 8].try_into().unwrap()) as usize;
        }
        assert_eq!(pos, data.len());
        assert_eq!(kinds.len(), ICNS_ENTRIES.len());
    }

    #[test]
    fn profile_svg_carries_name_and_colors() {
        let svg = profile_svg("R&D", &badge(), 120.0);
        assert!(svg.contains(">R&amp;D</text>"));
        assert!(svg.contains("fill=\"#000000\""));
        assert!(svg.contains("font-size=\"120\""));
        assert!(!svg.contains("{{"), "{svg}");
        profile("ACME", &badge()).unwrap();
        assert!(profile_png("ACME", &badge(), 64).unwrap().starts_with(b"\x89PNG"));
    }

    #[test]
    fn long_names_are_measured_wider() {
        let mut options = usvg::Options::default();
        options.fontdb_mut().load_system_fonts();
        let short = text_width("ACME", &options);
        let long = text_width("a-really-very-long-profile-name", &options);
        assert!(short > 1.0 && long > 3.0 * short, "{short} {long}");
        // Uppercase letters are wider than lowercase ones.
        assert!(short > text_width("acme", &options));
    }
}
