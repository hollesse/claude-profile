# PRD: `claude-profile` – getrennte Claude-Code-Profile (Subscription / Vertex)

Stand: 2026-11-20 · Status: Entwurf

## 1. Problem

Claude Code wird in zwei Kontexten genutzt, deren Daten strikt getrennt bleiben müssen:

| Kontext | Backend | Daten dürfen nur hier landen |
|---|---|---|
| Profil `subscription` | Claude-Subscription (`claude login`) | Subscription |
| Profil `vertex` | Vertex AI, typischerweise über ein Gateway oder einen Telemetrie-Proxy, eingerichtet durch ein externes Setup-Tool | Vertex |

Claude Code kennt keine Profile. Bisher wurde die Trennung durch eine virtuelle Maschine erreicht. Setup-Tools für solche Umgebungen schreiben häufig **fest** nach `$HOME/.claude/settings.json` (ohne `CLAUDE_CONFIG_DIR` zu beachten) und können das auch nach der Einrichtung erneut tun.

## 2. Ziele

1. Zwei (oder mehr) isolierte Config-Verzeichnisse: `~/.claude-subscription`, `~/.claude-vertex`.
2. `~/.claude` ist ein **Symlink** auf ein Profil, damit das Setup-Tool ohne Änderung direkt ins gewünschte Profil schreibt.
3. Ein Befehl `claude` (Shell-Funktion), der **nie** ohne explizites Profil startet.
4. Profilwahl per Umgebungsvariable `CLAUDE_PROFILE`, per interaktiver Auswahl oder per Ordner (direnv).
5. Eine CLI `claude-profile` zum Anlegen, Auflisten, Starten und Konfigurieren.
6. Funktioniert in bash und zsh auf macOS (Hauptziel) und Linux.
7. Claude Desktop (macOS) lässt sich pro Profil als eigene Instanz starten (`claude-profile desktop`), auch mehrere gleichzeitig, optional über einen eigenen Launcher mit Profilauswahl.

## 3. Nicht-Ziele

- Löschen von Profilen (bewusst nicht in v1, nur manuell).
- Verwalten von Inhalten der Profile (Settings, Plugins, CLAUDE.md).
- Ändern oder Patchen von Setup-Tools.
- Windows.

## 4. Layout

```
~/.claude-subscription/     # Profil subscription (CLAUDE_CONFIG_DIR)
~/.claude-vertex/           # Profil vertex
~/.claude -> ~/.claude-vertex          # Symlink (wählbar, siehe `default`)
~/.claude.json -> ~/.claude-vertex/.claude.json   # Symlink, folgt immer ~/.claude
~/.config/claude-profiles/profiles.json
```

`profiles.json`:

```json
{
  "link_target": "vertex",
  "claude_bin": "/Users/x/.local/bin/claude",
  "profiles": {
    "subscription": { "dir": "~/.claude-subscription", "forbid_env": ["CLAUDE_CODE_USE_VERTEX", "ANTHROPIC_VERTEX_*", "ANTHROPIC_BASE_URL", "GOOGLE_CLOUD_*", "OTEL_*"] },
    "vertex":       { "dir": "~/.claude-vertex",       "forbid_env": [] }
  }
}
```

- Profilname: `[A-Za-z0-9-]+`. Intern gilt die klein geschriebene Kennung (Registry, Verzeichnis, `CLAUDE_PROFILE`); die Schreibweise wird nur angezeigt. Eingaben werden ohne Rücksicht auf Groß-/Kleinschreibung gefunden.
- Verzeichnis per Konvention `~/.claude-<name>`.
- `forbid_env`: Variablen, die beim Start dieses Profils **entfernt** werden (Glob erlaubt). Schutz gegen Leaks aus Shell-Profil oder direnv.

## 5. Profilauflösung (Reihenfolge)

1. Explizites Argument (`claude-profile start <name>`).
2. `CLAUDE_PROFILE` (aus Shell oder direnv). Wert unbekannt → **Fehler**, kein Fallback.
3. Interaktive Auswahl (nur mit TTY).
4. Kein TTY und nichts gesetzt → Abbruch, Exit-Code 2, klare Fehlermeldung.

Es gibt **kein Default-Profil**.

## 6. Start-Verhalten (`run` / `start`)

- Setzt **immer** `CLAUDE_CONFIG_DIR=<profil.dir>`, auch für das Profil, auf das der Symlink zeigt. So ist `.claude.json` (liegt bei gesetztem `CLAUDE_CONFIG_DIR` im Profilordner) pro Profil getrennt, statt teils in `$HOME`.
- Entfernt `forbid_env` des Profils aus der Umgebung.
- Gibt eine Statuszeile aus: `claude-profile: vertex (~/.claude-vertex)` (abschaltbar).
- `exec` auf das echte `claude`-Binary (`claude_bin`, beim `add` ermittelt, kein Aufruf der eigenen Shell-Funktion), alle weiteren Argumente werden durchgereicht.

## 7. Befehle

### `claude-profile add [<name>]`
Interaktiv, idempotent, **löscht nie etwas**:
1. Name abfragen/validieren.
2. Fall A – `~/.claude` ist ein echtes Verzeichnis: Fragen „bestehendes `~/.claude` als Profil übernehmen (umbenennen nach `~/.claude-<name>`) oder neues leeres Profil anlegen?“. Bei Übernahme wird auch `~/.claude.json` in das Profilverzeichnis verschoben (zu verifizieren, siehe Abschnitt 11).
3. Fall B – `~/.claude` ist schon ein Symlink oder fehlt: neues Verzeichnis anlegen.
4. Frage, welches Profil der Symlink `~/.claude` zeigen soll (bei Erstanlage Pflicht, später optional).
5. Optional: `forbid_env`-Vorlage wählen (vorbelegt für „Subscription“, leer für „Vertex“).
6. Registry schreiben. Vor Umbenennungen Plan anzeigen und bestätigen lassen; `--dry-run` und `--yes` verfügbar.

### `claude-profile list`
Tabelle: Name, Verzeichnis, existiert?, `← Symlink` (wohin `~/.claude` zeigt), `← aktiv` (aus `CLAUDE_PROFILE`), `← direnv` (Wert im aktuellen Ordner). Warnt bei kaputtem Symlink oder fehlenden Verzeichnissen.

### `claude-profile start <name> [-- <claude-args>]`
Startet Claude mit diesem Profil (Abschnitt 6), ignoriert `CLAUDE_PROFILE`.

### `claude-profile default <name>`
Zeigt den Symlink `~/.claude` auf ein anderes Profil (mit Bestätigung; nur wenn `~/.claude` ein Symlink ist, sonst Abbruch).

### `claude-profile set <name>` / `claude-profile set --unset`
Schreibt/aktualisiert im **aktuellen Ordner** einen markierten Block in `.envrc`:

```
# BEGIN claude-profile
export CLAUDE_PROFILE=vertex
# END claude-profile
```

Prüft, ob `direnv` installiert und in der Shell gehookt ist, fragt vor `direnv allow`. Hinweis, `.envrc` nicht zu committen (Vorschlag: `.git/info/exclude`).

### `claude-profile replace-claude-command [--shell zsh|bash] [--remove]`
Trägt in `~/.zshrc` bzw. `~/.bashrc` einen markierten, idempotenten Block ein:

```
# BEGIN claude-profile
claude() { command claude-profile run -- "$@"; }
# END claude-profile
```

Zeigt vorher den Diff, legt ein Backup der rc-Datei an. `--remove` entfernt den Block wieder.

### `claude-profile desktop [<name>]` (macOS)
Startet Claude Desktop für ein Profil als **eigene Instanz**:

```
open -n -a Claude --env CLAUDE_CONFIG_DIR=<profil.dir> --args --user-data-dir="$HOME/Library/Application Support/Claude-<name>"
```

- Ohne `<name>` erscheint dieselbe Profilauswahl wie bei `run`.
- `--user-data-dir` trennt Login, Cache und App-Daten pro Profil; `-n` erzwingt eine neue Instanz, sodass **mehrere Profile gleichzeitig** laufen können.
- `CLAUDE_CONFIG_DIR` wird mitgegeben, damit der Code-Tab der Desktop-App nicht die Standard-Config `~/.claude` benutzt (zu verifizieren, Q7/Q8).
- Das Profil speichert optional `desktop_user_data_dir` in `profiles.json`.

### `claude-profile desktop-app <name>` / `claude-profile launcher` (macOS, optional)
- `desktop-app`: erzeugt pro Profil ein kleines, per Spotlight/Dock startbares App-Bundle in `/Applications` (anderer Ordner per `--dir`; z. B. „Claude (Vertex).app“), das `claude-profile desktop <name>` aufruft. Eigener Name (optional eigenes Icon), damit die Instanzen im Dock unterscheidbar sind.
- `launcher`: erzeugt „Claude Profiles.app“, das per Auswahldialog (`osascript`/`choose from list`) das Profil abfragt und dann `desktop <name>` startet.

### `claude-profile run -- [args]`
Interne Variante: Auflösung nach Abschnitt 5, dann Start nach Abschnitt 6. Wird von der Shell-Funktion benutzt.

## 8. Sicherheits-/Isolationsanforderungen

| ID | Anforderung |
|---|---|
| S1 | Kein Start ohne aufgelöstes Profil. |
| S2 | `CLAUDE_CONFIG_DIR` wird bei jedem Start explizit gesetzt. |
| S3 | `forbid_env` wird vor dem Start entfernt; vorhandene Treffer werden als Warnung ausgegeben. |
| S4 | Vertex-Settings (`CLAUDE_CODE_USE_VERTEX`, OTel, Auth-Refresh-Hooks …) stehen ausschließlich in der `settings.json` des Vertex-Profils, nie in der Shell-Umgebung. |
| S5 | Kein Dateiverlust: Umbenennen statt Löschen, Backups vor Änderungen an rc-Dateien. |
| S6 | `claude-profile` liest oder kopiert keine Credentials und sendet nichts an Dritte. |
| S7 | Fehlerhafte Eingaben (unbekanntes Profil, ungültiger Name) brechen ab statt zu raten. |

## 9. Interop mit Setup-Tools, die den Standardpfad benutzen

Gemeint sind Werkzeuge, die Claude Code für ein anderes Backend einrichten – zum Beispiel für ein Gateway oder einen Telemetrie-Proxy (OpenTelemetry) – und dabei den Config-Pfad nicht einstellbar machen.

Annahmen:

- Das Tool schreibt fest nach `~/.claude/settings.json`, typischerweise in den `env`-Block. Schreibt es die Datei direkt (ohne Rename), wird durch den Symlink hindurch ins Profil geschrieben.
- Das Tool kann die Datei nicht nur bei der Einrichtung schreiben, sondern auch später, etwa bei einer erneuten Anmeldung, einem Neustart oder einem Update. Wann genau, hängt vom Tool ab (Q6).
- **Konsequenz:** Der Symlink `~/.claude` → Vertex-Profil muss **dauerhaft** bleiben; ein kurzzeitiges Umbiegen nur für die Einrichtung ist nicht sicher.
- `claude-profile list` prüft den Symlink und warnt, wenn `~/.claude` kein Symlink mehr ist (stiller Bruch).
- Ein Telemetrie-Proxy oder anderer Hintergrunddienst läuft unabhängig vom Profil (global). Sitzungen im Subscription-Profil senden nichts an ihn, solange keine `OTEL_*`-Variablen gesetzt sind (S3).

## 10. Risiken

| ID | Risiko | Gegenmaßnahme |
|---|---|---|
| **R1** | Alles, was die Standard-Config `~/.claude` liest (Desktop-App ohne `CLAUDE_CONFIG_DIR`, IDE-Plugins, ein versehentliches `command claude`), bekommt per Symlink die **Vertex-Settings**. | Desktop wird nur noch über `claude-profile desktop` gestartet (setzt `CLAUDE_CONFIG_DIR`, eigenes `--user-data-dir`). Rest-Risiko: Apps, die nicht über die CLI starten (Q1, Q7). Der Symlink bleibt dauerhaft (Abschnitt 9). |
| R2 | macOS: Der Subscription-Login liegt im Keychain; der Eintragsname hängt (nach bisherigem Kenntnisstand) vom Config-Verzeichnis ab. Nach dem Umbenennen von `~/.claude` kann ein erneutes `claude login` nötig sein. | In `add` darauf hinweisen. Verifizieren (Q3). |
| R3 | Das Setup-Tool ersetzt den Symlink durch eine Datei (z. B. via Rename). | Warnung in `list`. |
| R4 | Shell-Funktion greift nicht in Skripten/nicht-interaktiven Shells. | Optionaler PATH-Shim (Q4). |
| R5 | `.envrc` mit `CLAUDE_PROFILE` wird versehentlich committet. | Hinweis in `set`. |
| R6 | Desktop-Instanzen sehen im Dock gleich aus; `claude://`-Login-Callbacks (OAuth) landen evtl. in der falschen Instanz. | Eigene App-Bundles mit Namen/Icon (`desktop-app`); Login einer Instanz einzeln durchführen (Q9). |

## 11. Offene Fragen / zu verifizieren

- **Q1** Liest die Desktop-App `~/.claude/settings.json` (und wendet `env` dort an)? Entscheidet, wohin der Symlink standardmäßig zeigt.
- **Q2 (geklärt, Claude Code 2.1.329)** Bei gesetztem `CLAUDE_CONFIG_DIR` liegt die `.claude.json` im Profilordner; ohne die Variable liest Claude Code `~/.claude.json` im Home (nicht `~/.claude/.claude.json`). Deshalb ist `~/.claude.json` ein Symlink auf die `.claude.json` des Profils, auf das `~/.claude` zeigt; `add` und `default` pflegen ihn, `list` warnt, wenn er fehlt. Claude Code schreibt durch den Symlink hindurch und erhält ihn (getestet, auch bei fehlender Zieldatei). Eine echte Datei an dieser Stelle wird ins Profil verschoben oder, falls das Profil schon eine hat, umbenannt – nie gelöscht.
- **Q3** Keychain-Verhalten auf macOS beim Umbenennen des Config-Verzeichnisses.
- **Q4** Shell-Funktion (wie gewünscht) oder zusätzlich/alternativ ein `claude`-Shim in einem PATH-Verzeichnis (funktioniert auch in Skripten, IDE-Terminals)?
- **Q5 (entschieden)** Implementierungssprache: **Rust** (eine native Binary; `clap`, `inquire`/`dialoguer`, `serde_json`; wenige Abhängigkeiten). Verteilung: lokal bauen (`cargo install --path .`) und ad-hoc signieren (`codesign -s -`); für eine Weitergabe wären Developer-ID-Signatur und Notarisierung nötig. Für den Spike (Q6–Q9) ist ein Shell-Prototyp zulässig.
- **Q6** Wann schreibt ein Setup-Tool erneut nach `settings.json` (nur bei der Einrichtung oder auch bei Neustart, erneuter Anmeldung, Update)? Ändert am Design nichts (Symlink bleibt dauerhaft), erklärt aber, wann Änderungen zu erwarten sind.
- **Q7** Liest die Desktop-App die Umgebungsvariable `CLAUDE_CONFIG_DIR` für ihren Code-Tab, und reicht `open --env` sie an die App weiter (macOS-Version)? Alternative: App-Binary direkt mit gesetzter Umgebung starten.
- **Q8** Liegt `claude_desktop_config.json` (MCP) bei `--user-data-dir` im neuen Ordner oder weiter im Standardordner?
- **Q9** Funktionieren zwei parallele Instanzen mit `open -n` + eigenem `--user-data-dir` (Single-Instance-Lock, `claude://`-Callbacks, Keychain-Eintrag „Claude Safe Storage“)?
- **Q10** Unterstützt die Desktop-App Vertex (Drittanbieter-Inferenz)? Wenn nein, ist die zweite Instanz nur ein getrennter Subscription-Login, kein Vertex-Profil.

## 12. Akzeptanzkriterien

1. `claude` ohne `CLAUDE_PROFILE` und mit TTY zeigt die Auswahl; ohne TTY bricht es mit Exit 2 ab.
2. `CLAUDE_PROFILE=subscription claude` startet mit `CLAUDE_CONFIG_DIR=~/.claude-subscription`, und `CLAUDE_CODE_USE_VERTEX` ist in diesem Prozess nicht gesetzt, auch wenn es in der Shell gesetzt ist.
3. `CLAUDE_PROFILE=foo claude` bricht mit klarer Fehlermeldung ab.
4. `claude-profile add` übernimmt ein bestehendes `~/.claude` per Umbenennen, ohne Daten zu verlieren, und legt den Symlink an.
5. `claude-profile list` zeigt Symlink-Ziel, aktives Profil und direnv-Wert korrekt an und warnt bei kaputtem Symlink.
6. `claude-profile set vertex` in einem Ordner macht dort `claude` ohne Rückfrage zum Vertex-Profil (nach `direnv allow`); in einem anderen Ordner bleibt die Auswahl.
7. `replace-claude-command` ist idempotent (zweimal ausführen = ein Block) und `--remove` stellt den Ursprungszustand her.
8. Nach Einrichtung oder Re-Authentifizierung über das Setup-Tool liegen die geänderten Settings im Vertex-Profil, und `~/.claude-subscription` ist unverändert.
9. Keine Aktion löscht Benutzerdaten.
10. `claude-profile desktop subscription` startet Claude Desktop mit `--user-data-dir=…/Claude-subscription` und `CLAUDE_CONFIG_DIR=~/.claude-subscription`; ein zweiter Aufruf `desktop vertex` startet daneben eine zweite Instanz mit eigenem Datenordner (vorbehaltlich Q7–Q9).
11. `claude-profile desktop` ohne Namen zeigt die Profilauswahl; ohne gültige Auswahl wird keine Instanz gestartet.
12. `claude-profile desktop-app <name>` erzeugt ein startbares Bundle in `/Applications`, das per Spotlight auffindbar ist und die richtige Instanz startet; `claude-profile launcher` erzeugt „Claude Profiles.app“ mit Auswahldialog.

## 13. Anforderungs-Check

| # | Anforderung | Abgedeckt durch | Status |
|---|---|---|---|
| 1 | Zwei getrennte Config-Ordner pro Backend | Abschnitt 4, `add` | ✅ |
| 2 | Symlink `~/.claude` auf ein wählbares Profil | Abschnitt 4, `add` Schritt 4, `default`, Abschnitt 9 (dauerhaft) | ✅, Rest-Risiko: Programme, die nicht über `claude-profile` starten und `~/.claude` lesen (R1, Q1, Q7) |
| 3 | Shell-Funktion `claude`, die die Profilvariable prüft | Abschnitt 5, 7 (`replace-claude-command`, `run`) | ✅ (Variable heißt `CLAUDE_PROFILE`) |
| 4 | Variable gesetzt → Start mit dem passenden Config-Dir | Abschnitt 5/6 | ✅ |
| 5 | Nicht gesetzt → Auswahl der Profile | Abschnitt 5 Punkt 3 | ✅ (ohne TTY: Abbruch statt Auswahl) |
| 6 | `claude-profile add <name>` interaktiv | Abschnitt 7 `add` | ✅ |
| 7 | Bestehendes `~/.claude` umbenennen oder neuer Pfad | `add` Fall A/B | ✅ |
| 8 | Frage, welches Profil der Standard-Symlink bekommt | `add` Schritt 4, `default` | ✅ |
| 9 | `claude-profile list` | Abschnitt 7 | ✅ |
| 10 | `claude-profile start <name>` | Abschnitt 7 | ✅ |
| 11 | `claude-profile replace-claude-command` (zshrc/bashrc) | Abschnitt 7 | ✅ |
| 12 | `claude-profile set` über direnv pro Ordner | Abschnitt 7 `set` | ✅ |
| 13 | Daten je Backend strikt getrennt | Abschnitt 8 (S1–S4), Akzeptanz 2 | ✅ für die CLI-Nutzung, ⚠️ nicht für Programme, die `~/.claude` direkt lesen (R1) |
| 14 | Funktioniert in CLI und Desktop | Abschnitt 7 (`desktop`, `desktop-app`, `launcher`), Akzeptanz 10–12 | ⚠️ ungeprüft: Q7–Q10 |
| 15 | Ohne virtuelle Maschine | Abschnitt 2 | ✅ (Isolation ist geringer als in einer VM) |
| 16 | Desktop über `open -n -a Claude --args --user-data-dir=…` umbiegen, wählbar beim Start | Abschnitt 7 `desktop` | ✅ |
| 17 | Eigener Launcher mit Profilauswahl für Desktop | Abschnitt 7 `launcher`, `desktop-app` | ✅ |
| 18 | Zwei Desktop-Instanzen gleichzeitig | Abschnitt 7 `desktop` (`-n`), Q9, R6 | ⚠️ ungeprüft: Q9 |
| 19 | Schreibpfad des Setup-Tools geklärt | Abschnitt 9, Q6 | ⚠️ hängt vom jeweiligen Tool ab (Q6); das Design geht vom ungünstigsten Fall aus |
