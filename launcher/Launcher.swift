// Window of “Claude Profiles.app”: shows the profiles with their icons and which starter app
// is already installed, installs missing ones and opens them.
//
// The app itself knows no profiles. It asks `claude-profile launcher-data` and calls
// `claude-profile desktop-app` and `open` for everything else. build.rs compiles this file,
// the binary is embedded in claude-profile and written into the bundle by
// `claude-profile launcher`.

import AppKit
import SwiftUI

struct Profile: Decodable, Identifiable {
    /// Lowercase ID
    let id: String
    /// Display name
    let name: String
    /// PNG of the profile icon
    let icon: String
    /// Path of the installed starter app, if there is one
    let app: String?

    var isInstalled: Bool { app != nil }
}

struct Payload: Decodable {
    let profiles: [Profile]
}

struct ToolError: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}

enum Tool {
    private static let environment = ProcessInfo.processInfo.environment

    /// Path of the claude-profile binary; `claude-profile launcher` writes it into the Info.plist.
    static let bin: String =
        environment["CLAUDE_PROFILE_BIN"]
        ?? Bundle.main.object(forInfoDictionaryKey: "ClaudeProfileBin") as? String
        ?? "/usr/local/bin/claude-profile"

    /// Folder for the starter apps: the one the launcher itself lives in.
    static let appsDir: String =
        environment["CLAUDE_PROFILE_APPS_DIR"]
        ?? Bundle.main.bundleURL.deletingLastPathComponent().path

    static func run(_ arguments: [String]) throws -> Data {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: bin)
        process.arguments = arguments
        let output = Pipe()
        let errors = Pipe()
        process.standardOutput = output
        process.standardError = errors
        do {
            try process.run()
        } catch {
            throw ToolError(message: "\(bin) cannot be started – recreate the launcher with `claude-profile launcher`.")
        }
        let data = output.fileHandleForReading.readDataToEndOfFile()
        let errorData = errors.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            let text = String(decoding: errorData, as: UTF8.self)
                .trimmingCharacters(in: .whitespacesAndNewlines)
            throw ToolError(message: text.isEmpty ? "claude-profile failed." : text)
        }
        return data
    }
}

final class Model: ObservableObject {
    @Published var profiles: [Profile] = []
    @Published var error: String?
    /// ID of the profile whose starter app is being installed
    @Published var installing: String?
    @Published var isLoaded = false

    func reload() {
        do {
            let data = try Tool.run(["launcher-data", "--dir", Tool.appsDir])
            profiles = try JSONDecoder().decode(Payload.self, from: data).profiles
        } catch {
            self.error = error.localizedDescription
        }
        isLoaded = true
    }

    func install(_ profile: Profile, thenOpen: Bool) {
        guard installing == nil else { return }
        installing = profile.id
        error = nil
        DispatchQueue.global(qos: .userInitiated).async {
            let result = Result { try Tool.run(["desktop-app", profile.id, "--dir", Tool.appsDir]) }
            DispatchQueue.main.async {
                self.installing = nil
                switch result {
                case .failure(let failure):
                    self.error = failure.localizedDescription
                case .success:
                    self.reload()
                    let installed = self.profiles.first { $0.id == profile.id && $0.isInstalled }
                    if thenOpen, let installed {
                        self.open(installed)
                    }
                }
            }
        }
    }

    /// Opens the profile's starter app; if it is missing, it is installed first.
    func open(_ profile: Profile) {
        guard let app = profile.app else {
            install(profile, thenOpen: true)
            return
        }
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/open")
        process.arguments = ["-n", app]
        do {
            try process.run()
            process.waitUntilExit()
        } catch {
            self.error = error.localizedDescription
            return
        }
        guard process.terminationStatus == 0 else {
            error = "\(app) cannot be opened."
            return
        }
        // The launcher has done its job and disappears from the Dock.
        NSApp.terminate(nil)
    }
}

struct Tile: View {
    let profile: Profile
    @ObservedObject var model: Model
    @State private var isHovering = false

    private var icon: NSImage {
        NSImage(contentsOfFile: profile.icon) ?? NSApp.applicationIconImage
    }

    private var isInstalling: Bool { model.installing == profile.id }

    var body: some View {
        VStack(spacing: 10) {
            Button {
                model.open(profile)
            } label: {
                Image(nsImage: icon)
                    .resizable()
                    .interpolation(.high)
                    .frame(width: 116, height: 116)
                    .saturation(profile.isInstalled ? 1 : 0.35)
                    .opacity(profile.isInstalled ? 1 : 0.7)
                    .scaleEffect(isHovering ? 1.05 : 1)
                    .animation(.easeOut(duration: 0.12), value: isHovering)
            }
            .buttonStyle(.plain)
            .help(profile.isInstalled ? "Open \(profile.name)" : "Install and open \(profile.name)")

            Text(profile.name)
                .font(.headline)
                .lineLimit(1)

            if profile.isInstalled {
                Label("Installed", systemImage: "checkmark.circle.fill")
                    .font(.caption)
                    .foregroundStyle(.green)
            } else {
                Label("Not installed", systemImage: "arrow.down.circle")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            if isInstalling {
                ProgressView()
                    .controlSize(.small)
                    .frame(height: 46)
            } else if profile.isInstalled {
                Button("Open") { model.open(profile) }
                    .buttonStyle(.borderedProminent)
                    .frame(height: 46, alignment: .top)
            } else {
                VStack(spacing: 4) {
                    Button("Install & open") { model.open(profile) }
                        .buttonStyle(.borderedProminent)
                    Button("Install only") { model.install(profile, thenOpen: false) }
                        .buttonStyle(.link)
                        .font(.caption)
                }
                .frame(height: 46, alignment: .top)
            }
        }
        .padding(.vertical, 16)
        .frame(width: Layout.tileWidth)
        .background(
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(Color.primary.opacity(isHovering ? 0.09 : 0.045))
        )
        .onHover { isHovering = $0 }
        .disabled(model.installing != nil)
    }
}

enum Layout {
    static let tileWidth: CGFloat = 190
    static let spacing: CGFloat = 14
    static let padding: CGFloat = 24
    static let maxColumns = 4

    static func width(for count: Int) -> CGFloat {
        let columns = CGFloat(min(max(count, 2), maxColumns))
        return columns * tileWidth + (columns - 1) * spacing + 2 * padding
    }
}

struct ContentView: View {
    @ObservedObject var model: Model

    private var columns: [GridItem] {
        let count = min(max(model.profiles.count, 1), Layout.maxColumns)
        return Array(repeating: GridItem(.fixed(Layout.tileWidth), spacing: Layout.spacing), count: count)
    }

    private var shownDir: String {
        (Tool.appsDir as NSString).abbreviatingWithTildeInPath
    }

    var body: some View {
        VStack(spacing: 18) {
            VStack(spacing: 4) {
                Text("Open Claude Desktop")
                    .font(.title2.weight(.semibold))
                Text("Choose a profile. Starter apps live in \(shownDir).")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

            if model.profiles.isEmpty && model.isLoaded && model.error == nil {
                Text("No profiles yet.\nCreate one in the terminal: claude-profile add")
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.secondary)
                    .padding(.vertical, 30)
            } else {
                LazyVGrid(columns: columns, spacing: Layout.spacing) {
                    ForEach(model.profiles) { profile in
                        Tile(profile: profile, model: model)
                    }
                }
            }

            if let error = model.error {
                Label(error, systemImage: "exclamationmark.triangle.fill")
                    .font(.callout)
                    .foregroundStyle(.red)
                    .fixedSize(horizontal: false, vertical: true)
                    .textSelection(.enabled)
            }
        }
        .padding(Layout.padding)
        .frame(width: Layout.width(for: model.profiles.count))
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
        // For visual checks during development: save the window content as PNG and quit.
        if let path = ProcessInfo.processInfo.environment["CLAUDE_PROFILE_LAUNCHER_SNAPSHOT"] {
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) {
                Self.snapshot(to: path)
                NSApp.terminate(nil)
            }
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }

    private static func snapshot(to path: String) {
        guard let view = NSApp.windows.first(where: { $0.isVisible })?.contentView,
            let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds)
        else { return }
        view.cacheDisplay(in: view.bounds, to: bitmap)
        try? bitmap.representation(using: .png, properties: [:])?
            .write(to: URL(fileURLWithPath: path))
    }
}

@main
struct LauncherApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @StateObject private var model = Model()

    var body: some Scene {
        WindowGroup("Claude Profiles") {
            ContentView(model: model)
                .onAppear { model.reload() }
        }
        .windowResizability(.contentSize)
        .commands {
            // One window is enough.
            CommandGroup(replacing: .newItem) {}
        }
    }
}
