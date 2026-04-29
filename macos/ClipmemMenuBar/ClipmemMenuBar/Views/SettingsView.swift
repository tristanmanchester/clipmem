import SwiftUI

struct ClipmemSettingsView: View {
    let appModel: AppModel

    @AppStorage(PreferenceKey.binaryPathOverride) private var binaryPathOverride = ""
    @AppStorage(PreferenceKey.databasePathOverride) private var databasePathOverride = ""
    @AppStorage(PreferenceKey.defaultRecentHours) private var defaultRecentHours = 24
    @AppStorage(PreferenceKey.defaultQueryMode) private var defaultQueryMode = QueryMode.recent.rawValue
    @AppStorage(PreferenceKey.hotkeyEnabled) private var hotkeyEnabled = true
    @State private var selectedTab: SettingsTab = .general
    @State private var handledSettingsOpenRequestID = 0
    @State private var newIgnoredBundleID = ""
    @State private var retentionValue = "forever"
    @FocusState private var retentionFieldFocused: Bool
    @State private var confirmRetention = false
    @State private var confirmCompact = false
    @State private var confirmCompressImages = false
    @State private var showManualPurge = false

    var body: some View {
        TabView(selection: $selectedTab) {
            generalTab
                .tag(SettingsTab.general)
                .tabItem { Label(SettingsTab.general.title, systemImage: SettingsTab.general.symbol) }

            storageTab
                .tag(SettingsTab.storage)
                .tabItem { Label(SettingsTab.storage.title, systemImage: SettingsTab.storage.symbol) }

            captureTab
                .tag(SettingsTab.capture)
                .tabItem { Label(SettingsTab.capture.title, systemImage: SettingsTab.capture.symbol) }

            ignoredAppsTab
                .tag(SettingsTab.ignoredApps)
                .tabItem { Label(SettingsTab.ignoredApps.title, systemImage: SettingsTab.ignoredApps.symbol) }

            diagnosticsTab
                .tag(SettingsTab.diagnostics)
                .tabItem { Label(SettingsTab.diagnostics.title, systemImage: SettingsTab.diagnostics.symbol) }

            privacyTab
                .tag(SettingsTab.privacy)
                .tabItem { Label(SettingsTab.privacy.title, systemImage: SettingsTab.privacy.symbol) }
        }
        .overlay(alignment: .bottom) {
            ActionFeedbackOverlay(message: appModel.actionMessage, transitionEdge: .bottom)
                .padding(.bottom, Spacing.lg)
        }
        .task {
            applyPendingSettingsOpenRequestIfNeeded()
            await refreshSettingsSurface()
        }
        .onChange(of: appModel.pendingSettingsOpenRequest?.id) {
            applyPendingSettingsOpenRequestIfNeeded()
        }
        .onChange(of: appModel.settingsReport?.retention) { _, value in
            guard retentionFieldFocused == false else { return }
            retentionValue = value ?? "forever"
        }
    }

    // MARK: - General Tab

    private var generalTab: some View {
        Form {
            Section("Paths") {
                TextField("clipmem binary", text: $binaryPathOverride)
                TextField("Database path", text: $databasePathOverride)
                Text("Leave blank to use the default paths.")
                    .font(DesignType.rowMeta)
                    .foregroundStyle(.secondary)
            }

            Section("Preferences") {
                Stepper("Recent window: \(defaultRecentHours) hours", value: $defaultRecentHours, in: 1...720)
                Picker("Default mode", selection: defaultDisplayModeBinding) {
                    ForEach(DisplayMode.allCases) { mode in
                        Text(mode.title).tag(mode)
                    }
                }
                Toggle("Enable Option-Shift-V global hotkey", isOn: $hotkeyEnabled)
                if let message = appModel.hotkeyMessage {
                    Text(message)
                        .foregroundStyle(.orange)
                }
                Toggle("Open Clipmem at login", isOn: launchAtLoginBinding)
                if let message = appModel.launchAtLoginError?.message {
                    Text(message)
                        .foregroundStyle(.orange)
                } else if let message = appModel.launchAtLoginStatus.message {
                    Text(message)
                        .foregroundStyle(.secondary)
                }
            }

            updateSettingsSection
        }
        .formStyle(.grouped)
        .padding()
    }

    // MARK: - Storage Tab

    private var storageTab: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Spacing.lg) {
                GroupBox("Archive Storage") {
                    VStack(alignment: .leading, spacing: Spacing.md) {
                        Grid(alignment: .leading, horizontalSpacing: Spacing.md, verticalSpacing: Spacing.sm) {
                            FieldRow(title: "Database size", value: databaseSizeDescription, showPlaceholder: true)
                            FieldRow(title: "Database path", value: databasePathDescription, showPlaceholder: true)
                            FieldRow(title: "Retention", value: appModel.settingsReport?.retention, showPlaceholder: true)
                        }

                        Text("Copied screenshots and image-heavy clips can take significant disk space. Compression keeps the archive searchable while reducing eligible stored image bytes.")
                            .font(DesignType.bodySecondary)
                            .foregroundStyle(.secondary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }

                StorageActionRow(
                    title: "Compress Images",
                    detail: "Convert eligible stored screenshots and images to lossless WebP when it saves space, then compact the database.",
                    systemImage: "photo.stack",
                    buttonTitle: "Compress Images",
                    isRunning: appModel.isRunningAction,
                    progress: appModel.imageOptimizationProgress
                ) {
                    confirmCompressImages = true
                }

                StorageActionRow(
                    title: "Compact Database",
                    detail: "Return unused SQLite and WAL pages to disk without changing clipboard history.",
                    systemImage: "archivebox",
                    buttonTitle: "Compact Database",
                    isRunning: appModel.isRunningAction
                ) {
                    confirmCompact = true
                }

                StorageActionRow(
                    title: "Purge Old History",
                    detail: "Preview matching snapshots before permanently deleting old clipboard history.",
                    systemImage: "trash",
                    buttonTitle: "Purge Old History...",
                    role: .destructive,
                    isRunning: appModel.isRunningAction
                ) {
                    showManualPurge = true
                }
            }
            .padding()
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .confirmationDialog("Compress stored images?", isPresented: $confirmCompressImages) {
            Button("Compress Images") {
                Task { await appModel.optimizeImages() }
            }
            Button("Keep Images As Is", role: .cancel) {}
        } message: {
            Text("Clipmem converts eligible screenshots and images to lossless WebP only when it saves space. Image content stays visually identical, already processed images are skipped, and the database is compacted afterward.")
        }
        .confirmationDialog("Compact database?", isPresented: $confirmCompact) {
            Button("Compact Database") {
                Task { await appModel.compactDatabase() }
            }
            Button("Leave Database As Is", role: .cancel) {}
        } message: {
            Text("This reclaims unused SQLite and WAL disk space without deleting clipboard history. The operation may need temporary disk space while SQLite rebuilds the database.")
        }
        .sheet(isPresented: $showManualPurge) {
            ManualPurgeSheet(appModel: appModel, initialDuration: retentionValue)
        }
    }

    // MARK: - Capture Tab

    private var captureTab: some View {
        Form {
            Toggle("Pause capture", isOn: pauseBinding)
            Toggle("API-key filter", isOn: apiKeyFilterBinding)
            Toggle("OCR for copied images", isOn: ocrBinding)
            LabeledContent("Retention") {
                HStack {
                    TextField("Duration", text: $retentionValue)
                        .textFieldStyle(.roundedBorder)
                        .focused($retentionFieldFocused)
                    Button("Apply") {
                        confirmRetention = true
                    }
                    .buttonStyle(.borderedProminent)
                }
            }
            Text("Use values like 30d, 12h, 15m, or forever.")
                .font(DesignType.rowMeta)
                .foregroundStyle(.secondary)
        }
        .formStyle(.grouped)
        .padding()
        .confirmationDialog("Apply retention policy?", isPresented: $confirmRetention) {
            Button("Apply Retention") {
                Task {
                    await appModel.runAction(.settingsRetention(retentionValue), successMessage: "Retention updated")
                    await appModel.refreshSettings()
                }
            }
            Button("Keep Current Retention", role: .cancel) {}
        } message: {
            Text("Items older than this threshold may be purged during the next cleanup cycle.")
        }
    }

    // MARK: - Ignored Apps Tab

    private var ignoredAppsTab: some View {
        VStack(alignment: .leading, spacing: Spacing.md) {
            HStack {
                TextField("App identifier (for example, com.apple.Safari)", text: $newIgnoredBundleID)
                Button("Add", systemImage: "plus") {
                    addIgnoredBundleID()
                }
                .disabled(newIgnoredBundleID.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
            let ignoredApps = appModel.settingsReport?.ignoredBundleIds ?? []
            if ignoredApps.isEmpty {
                EmptyStateView(
                    title: "No apps ignored",
                    detail: "Add bundle identifiers above to exclude apps from capture.",
                    symbol: "app.badge",
                    compact: true
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List {
                    ForEach(ignoredApps, id: \.self) { bundleID in
                        HStack {
                            Text(bundleID)
                                .textSelection(.enabled)
                            Spacer()
                            Button("Remove", systemImage: "minus.circle") {
                                Task {
                                    await appModel.runAction(.settingsIgnoreRemove(bundleID))
                                    await appModel.refreshSettings()
                                }
                            }
                            .labelStyle(.iconOnly)
                            .help("Remove \(bundleID)")
                        }
                    }
                }
            }
            Text("The menu bar app adds io.openclaw.clipmem.menubar by default to avoid self-capture noise.")
                .font(DesignType.rowMeta)
                .foregroundStyle(.secondary)
        }
        .padding()
    }

    // MARK: - Diagnostics Tab

    private var diagnosticsTab: some View {
        DiagnosticsView(appModel: appModel)
    }

    // MARK: - Privacy Tab

    private var privacyTab: some View {
        VStack(alignment: .leading, spacing: Spacing.lg) {
            GroupBox {
                Label("Your clipboard archive stays on this Mac.", systemImage: "checkmark.shield")
            }
            GroupBox("Storage") {
                VStack(alignment: .leading, spacing: Spacing.sm) {
                    Text("The database defaults to ~/Library/Application Support/clipmem/. See Settings > Diagnostics for the exact path.")
                    Text("The database is not encrypted. Enable FileVault for at-rest protection.")
                        .foregroundStyle(.secondary)
                }
            }
            GroupBox("What Gets Captured") {
                VStack(alignment: .leading, spacing: Spacing.sm) {
                    Text("Images and PDFs are stored as-is unless you use Settings > Storage to compress eligible images.")
                    Text("Text content is not processed by AI.")
                    Text("Search is keyword-based, not AI or cloud-powered.")
                    Text("The \"Copied while in\" label is a best guess based on the active app.")
                        .foregroundStyle(.secondary)
                }
            }
            Spacer()
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    // MARK: - Updates Section

    @ViewBuilder
    private var updateSettingsSection: some View {
        Section("Updates") {
            LabeledContent("Current version", value: appModel.updateStatus.currentVersion)
            LabeledContent("Latest checked version", value: appModel.updateStatus.latestVersion ?? "Not checked")
            LabeledContent("Last checked", value: lastUpdateCheckDescription)

            HStack {
                Button("Check for Updates", systemImage: "arrow.clockwise") {
                    Task { await appModel.checkForUpdates() }
                }
                .disabled(appModel.updateStatus.isChecking)
                if appModel.updateStatus.isChecking {
                    ProgressView()
                        .controlSize(.small)
                }
            }

            if appModel.updateStatus.isUpdateAvailable {
                if appModel.updateStatus.shouldShowHomebrewCommand {
                    VStack(alignment: .leading, spacing: Spacing.sm) {
                        Text("Update with Homebrew")
                            .font(DesignType.sectionHeader)
                        Text(UpdateChecker.homebrewUpgradeCommand)
                            .font(DesignType.rowMeta.monospaced())
                            .textSelection(.enabled)
                        Button("Copy Upgrade Command", systemImage: "doc.on.doc") {
                            appModel.copyUpgradeCommand()
                        }
                    }
                } else {
                    VStack(alignment: .leading, spacing: Spacing.sm) {
                        Text("Download from GitHub Releases")
                            .font(DesignType.sectionHeader)
                        Button("Open Release", systemImage: "arrow.up.right.square") {
                            appModel.openUpdateRelease()
                        }
                        .disabled(appModel.updateStatus.releaseURL == nil)
                    }
                }
            }

            if let message = appModel.updateStatus.errorMessage {
                Text(message)
                    .foregroundStyle(.orange)
            }
        }
    }

    // MARK: - Bindings

    private var defaultDisplayModeBinding: Binding<DisplayMode> {
        Binding {
            let mode = QueryMode(rawValue: defaultQueryMode) ?? .recent
            return DisplayMode.from(queryMode: mode).displayMode
        } set: { newDisplayMode in
            switch newDisplayMode {
            case .search: defaultQueryMode = QueryMode.recall.rawValue
            case .recent: defaultQueryMode = QueryMode.recent.rawValue
            case .timeline: defaultQueryMode = QueryMode.timeline.rawValue
            }
        }
    }

    private var pauseBinding: Binding<Bool> {
        Binding {
            appModel.settingsReport?.paused ?? false
        } set: { value in
            Task {
                await appModel.runAction(.settingsPause(value), successMessage: value ? "Capture paused" : "Capture resumed")
                await appModel.refreshSettings()
            }
        }
    }

    private var apiKeyFilterBinding: Binding<Bool> {
        Binding {
            appModel.settingsReport?.apiKeyFilterEnabled ?? false
        } set: { value in
            Task {
                await appModel.runAction(.settingsAPIKeyFilter(value), successMessage: value ? "API-key filter enabled" : "API-key filter disabled")
                await appModel.refreshSettings()
            }
        }
    }

    private var ocrBinding: Binding<Bool> {
        Binding {
            appModel.settingsReport?.ocrEnabled ?? false
        } set: { value in
            Task {
                await appModel.runAction(.settingsOCR(value), successMessage: value ? "OCR enabled" : "OCR disabled")
                await appModel.refreshSettings()
            }
        }
    }

    private var launchAtLoginBinding: Binding<Bool> {
        Binding {
            appModel.launchAtLoginEnabled
        } set: { value in
            appModel.setLaunchAtLoginEnabled(value)
        }
    }

    private var databaseSizeDescription: String? {
        DisplayFormatters.byteCount(appModel.serviceStatus?.dbSizeBytes)
    }

    private var databasePathDescription: String? {
        appModel.serviceStatus?.dbPath ?? databasePathOverride
    }

    private var lastUpdateCheckDescription: String {
        guard let lastCheckedAt = appModel.updateStatus.lastCheckedAt else {
            return "Never"
        }
        return lastCheckedAt.formatted(date: .abbreviated, time: .shortened)
    }

    // MARK: - Helpers

    private func refreshSettingsSurface() async {
        await appModel.refreshSettings()
        await appModel.refreshStatus()
        retentionValue = appModel.settingsReport?.retention ?? "forever"
    }

    private func applyPendingSettingsOpenRequestIfNeeded() {
        guard let request = appModel.pendingSettingsOpenRequest else { return }
        guard request.id != handledSettingsOpenRequestID else { return }
        handledSettingsOpenRequestID = request.id
        selectedTab = request.tab
    }

    private func addIgnoredBundleID() {
        let value = newIgnoredBundleID.trimmingCharacters(in: .whitespacesAndNewlines)
        guard value.isEmpty == false else { return }
        Task {
            await appModel.runAction(.settingsIgnoreAdd(value), successMessage: "App ignored")
            newIgnoredBundleID = ""
            await appModel.refreshSettings()
        }
    }
}

// MARK: - Storage Action Row

private struct StorageActionRow: View {
    let title: String
    let detail: String
    let systemImage: String
    let buttonTitle: String
    var role: ButtonRole?
    var isRunning = false
    var progress: ImageOptimizationProgressState?
    let action: () -> Void

    var body: some View {
        HStack(alignment: .center, spacing: Spacing.md) {
            Image(systemName: systemImage)
                .font(.title3)
                .foregroundStyle(isDestructive ? .red : .blue)
                .frame(width: 28)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: Spacing.xs) {
                Text(title)
                    .font(DesignType.sectionHeader)
                Text(detail)
                    .font(DesignType.bodySecondary)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                if let progress {
                    VStack(alignment: .leading, spacing: Spacing.xs) {
                        if let fractionCompleted = progress.fractionCompleted {
                            ProgressView(value: fractionCompleted)
                                .progressViewStyle(.linear)
                                .frame(maxWidth: 320)
                        } else {
                            ProgressView()
                                .controlSize(.small)
                        }
                        Text(progress.statusText)
                            .font(DesignType.rowMeta.weight(.medium))
                            .foregroundStyle(.primary)
                        Text(progress.detailText)
                            .font(DesignType.rowMeta)
                            .foregroundStyle(.secondary)
                    }
                    .padding(.top, Spacing.xs)
                }
            }

            Spacer(minLength: Spacing.lg)

            if isRunning && progress == nil {
                ProgressView()
                    .controlSize(.small)
            }

            Button(buttonTitle, role: role, action: action)
                .buttonStyle(.borderedProminent)
                .controlSize(.regular)
                .fixedSize(horizontal: true, vertical: false)
                .disabled(isRunning)
        }
        .padding(Spacing.lg)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.regularMaterial, in: .rect(cornerRadius: DesignRadius.md))
        .pressable()
        .accessibilityElement(children: .combine)
    }

    private var isDestructive: Bool {
        role != nil
    }
}
