# profiles.json

The registry is in `~/.config/claude-profiles/profiles.json`. `claude-profile` writes it itself; you can edit it by hand, for example to adjust `forbid_env`.

```json
{
  "link_target": "vertex",
  "claude_bin": "/Users/you/.local/bin/claude",
  "profiles": {
    "subscription": {
      "dir": "~/.claude-subscription",
      "forbid_env": [
        "CLAUDE_CODE_USE_VERTEX",
        "ANTHROPIC_VERTEX_*",
        "ANTHROPIC_BASE_URL",
        "GOOGLE_CLOUD_*",
        "OTEL_*"
      ]
    },
    "vertex": {
      "name": "Vertex",
      "dir": "~/.claude-vertex",
      "forbid_env": [],
      "badge": { "bg": "#f5a623", "fg": "#000000" }
    }
  }
}
```

## Fields

| Field | Meaning |
|---|---|
| `link_target` | Profile that `~/.claude` was last set to. The actual symlink is authoritative; `list` reads it directly. |
| `claude_bin` | Path of the real `claude` binary, determined on the first `add`. If the file is no longer executable, `claude` is searched on the `PATH`. |
| `profiles.<name>` | The profile's ID: the name in lowercase. A key with uppercase letters is converted on the next call. See [Profile names](commands.md#profile-names). |
| `profiles.<name>.name` | Optional: display name, for example `ACME`. It may differ from the ID only in case. If the entry is missing, the ID is displayed. Set it with [`claude-profile rename`](commands.md#rename). |
| `profiles.<name>.dir` | The profile's config directory. `~` stands for the home directory. |
| `profiles.<name>.forbid_env` | Variables that are removed when this profile starts. |
| `profiles.<name>.badge` | Optional: background (`bg`) and text color (`fg`) of the profile badge as `#rrggbb`. If the entry is missing, the profile gets a color derived from its name. Set it with [`claude-profile badge`](commands.md#badge). |
| `profiles.<name>.desktop_user_data_dir` | Optional: Claude Desktop data folder for this profile. Default: `~/Library/Application Support/Claude-<name>`. |

## forbid_env

Each entry is a variable name or a pattern with `*` as a placeholder for any number of characters:

| Pattern | Matches |
|---|---|
| `ANTHROPIC_BASE_URL` | exactly this variable |
| `OTEL_*` | `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME`, … |
| `*_VERTEX_*` | `ANTHROPIC_VERTEX_PROJECT_ID`, … |

`forbid_env` protects against variables that would leak into the wrong profile from the shell configuration or an `.envrc`. It only affects the **environment at start**. Entries in the `env` block of a profile's `settings.json` are not affected – that is where profile-specific settings belong.

The `subscription` template contains the five entries from the example above. If you use further variables for another backend (such as `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN` or `CLOUD_ML_REGION`), add them by hand.

## Renaming or removing a profile

You change only the display name with [`claude-profile rename`](commands.md#rename). There is no command for a different name or for removing a profile. To remove one, delete its entry under `profiles`; the directory stays. If `~/.claude` points to this profile, switch first with `claude-profile default <other>`.
