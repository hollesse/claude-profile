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
