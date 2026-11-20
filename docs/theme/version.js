// Version tag at the top right of the menu bar.
// Must match the version in Cargo.toml – a test (tests/docs.rs) checks this.
const CLAUDE_PROFILE_VERSION = "0.2.0";

(function () {
    const buttons = document.querySelector(".menu-bar .right-buttons");
    if (!buttons || buttons.querySelector(".version-tag")) {
        return;
    }
    const tag = document.createElement("span");
    tag.className = "version-tag";
    tag.textContent = "v" + CLAUDE_PROFILE_VERSION;
    tag.title = "claude-profile " + CLAUDE_PROFILE_VERSION;
    buttons.prepend(tag);
})();
