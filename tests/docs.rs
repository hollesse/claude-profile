//! Keeps the docs in sync with the code.

#[test]
fn docs_version_tag_matches_cargo_version() {
    let js = include_str!("../docs/theme/version.js");
    let expected = format!(
        "const CLAUDE_PROFILE_VERSION = \"{}\";",
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        js.contains(&expected),
        "docs/theme/version.js does not match the version in Cargo.toml – expected: {expected}"
    );
}

/// The docs show the launcher icon as logo and favicon; both are copies of the asset.
#[test]
fn docs_logo_matches_launcher_icon() {
    let icon = include_str!("../assets/launcher-icon.svg");
    assert_eq!(include_str!("../docs/src/assets/logo.svg"), icon, "docs/src/assets/logo.svg is out of date");
    assert_eq!(include_str!("../docs/theme/favicon.svg"), icon, "docs/theme/favicon.svg is out of date");
}
