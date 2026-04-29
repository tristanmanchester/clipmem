import SwiftUI

struct DiagnosticsView: View {
    let appModel: AppModel

    @State private var confirmUninstall = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Spacing.lg) {
                StatusBadge(state: appModel.healthState)

                if let error = appModel.lastError {
                    ErrorBanner(message: error.message, recovery: error.recovery)
                }

                GroupBox("Service") {
                    VStack(alignment: .leading, spacing: Spacing.sm) {
                        LabeledContent("Status", value: appModel.healthState.title)
                            .font(DesignType.bodySecondary)
                        DiagnosticsActionButton("Setup", systemImage: "wrench.and.screwdriver") {
                            Task { await appModel.runSetup() }
                        }
                        DiagnosticsActionButton("Start", systemImage: "play.fill") {
                            Task { await appModel.serviceAction("start") }
                        }
                        DiagnosticsActionButton("Stop", systemImage: "stop.fill") {
                            Task { await appModel.serviceAction("stop") }
                        }
                        Divider()
                        Button("Uninstall Service", role: .destructive) {
                            confirmUninstall = true
                        }
                        .disabled(appModel.isRunningAction)
                    }
                    .disabled(appModel.isRunningAction)
                }

                GroupBox("Binary and Database") {
                    VStack(alignment: .leading, spacing: Spacing.md) {
                        Grid(alignment: .leading, horizontalSpacing: Spacing.md, verticalSpacing: Spacing.sm) {
                            FieldRow(title: "Binary", value: appModel.client.resolvedBinaryPath() ?? "Not found", showPlaceholder: true)
                            FieldRow(title: "Watcher binary", value: appModel.serviceStatus?.watcherBinaryPath, showPlaceholder: true)
                            FieldRow(title: "Database", value: appModel.serviceStatus?.dbPath, showPlaceholder: true)
                            FieldRow(title: "Service method", value: appModel.serviceStatus?.preferredProvider.rawValue, showPlaceholder: true)
                            FieldRow(title: "Latest Capture", value: appModel.serviceStatus?.recentCaptureAt, showPlaceholder: true)
                            FieldRow(title: "Retention", value: appModel.serviceStatus?.retention, showPlaceholder: true)
                        }
                        if appModel.serviceStatus?.watcherBinaryMismatch == true,
                           let note = appModel.serviceStatus?.watcherBinaryMismatchNote {
                            Label(note, systemImage: "exclamationmark.triangle.fill")
                                .foregroundStyle(.orange)
                                .font(DesignType.bodySecondary)
                                .textSelection(.enabled)
                        }
                        DiagnosticsActionButton("Open Logs Folder", systemImage: "folder") {
                            appModel.openLogsFolder()
                        }
                        .disabled(appModel.serviceStatus?.logPaths.isEmpty != false)
                    }
                }

                GroupBox("Doctor") {
                    Grid(alignment: .leading, horizontalSpacing: Spacing.md, verticalSpacing: Spacing.sm) {
                        FieldRow(title: "SQLite", value: appModel.doctorReport?.sqliteVersion, showPlaceholder: true)
                        FieldRow(title: "Database mode", value: appModel.doctorReport?.journalMode, showPlaceholder: true)
                        FieldRow(title: "Full-text search", value: appModel.doctorReport?.fts5CreateVirtualTableOk.map { $0 ? "Available" : "Not available" }, showPlaceholder: true)
                    }
                    Button("Run Doctor", systemImage: "stethoscope") {
                        Task { await appModel.refreshDoctor() }
                    }
                    .padding(.top, Spacing.sm)
                }

                GroupBox("Agent Integration") {
                    VStack(alignment: .leading, spacing: Spacing.sm) {
                        DiagnosticsActionButton("Copy Context Command", systemImage: "doc.on.doc") {
                            appModel.copyAgentContextCommand()
                        }
                        DiagnosticsActionButton("Copy Skill Install Command", systemImage: "square.and.arrow.down") {
                            appModel.copyAgentSkillInstallCommand()
                        }
                        DiagnosticsActionButton("Copy OpenClaw Doctor Command", systemImage: "stethoscope") {
                            appModel.copyAgentOpenClawDoctorCommand()
                        }
                        DiagnosticsActionButton("Copy Hermes Doctor Command", systemImage: "stethoscope") {
                            appModel.copyAgentHermesDoctorCommand()
                        }
                        DiagnosticsActionButton("Copy Print Skill Command", systemImage: "doc.text.magnifyingglass") {
                            appModel.copyAgentPrintSkillCommand()
                        }
                        DiagnosticsActionButton("Copy Capability Map Command", systemImage: "map") {
                            appModel.copyAgentCapabilityMapCommand()
                        }
                        Text("Agents should start with context before multi-step recovery, use doctor commands when setup looks stale, and compose JSON primitives before broad workflows.")
                            .font(DesignType.bodySecondary)
                            .foregroundStyle(.secondary)
                            .fixedSize(horizontal: false, vertical: true)
                        Text("Example prompts: find the command I copied yesterday; recover the Safari URL I copied; show snapshot 128 with provenance.")
                            .font(DesignType.rowMeta)
                            .foregroundStyle(.secondary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }

                if let notes = appModel.serviceStatus?.notes, notes.isEmpty == false {
                    GroupBox("Notes") {
                        VStack(alignment: .leading, spacing: Spacing.sm) {
                            ForEach(notes, id: \.self) { note in
                                Text(note)
                                    .foregroundStyle(.secondary)
                            }
                        }
                    }
                }
            }
            .padding()
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .confirmationDialog("Uninstall the clipmem service?", isPresented: $confirmUninstall) {
            Button("Uninstall Service", role: .destructive) {
                Task { await appModel.serviceAction("uninstall") }
            }
            Button("Keep Service", role: .cancel) {}
        } message: {
            Text("This removes the LaunchAgent or Homebrew service registration. Your clipboard database is preserved.")
        }
        .task {
            await appModel.refreshDoctor()
        }
    }
}

private struct DiagnosticsActionButton: View {
    let title: String
    let systemImage: String
    let action: () -> Void

    init(_ title: String, systemImage: String, action: @escaping () -> Void) {
        self.title = title
        self.systemImage = systemImage
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            Label(title, systemImage: systemImage)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .buttonStyle(.bordered)
        .controlSize(.regular)
    }
}
