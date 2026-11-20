# Homebrew formula. The release workflow (.github/workflows/release.yml) updates `url` and
# `sha256` after each release – do not edit them by hand.
#
#   brew tap hollesse/claude-profile https://github.com/hollesse/claude-profile
#   brew install claude-profile
class ClaudeProfile < Formula
  desc "Separate Claude Code profiles on one machine"
  homepage "https://hollesse.github.io/claude-profile/"
  url "https://github.com/hollesse/claude-profile/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  head "https://github.com/hollesse/claude-profile.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/claude-profile --version")
    assert_match "No profiles yet", shell_output("#{bin}/claude-profile list")
  end
end
