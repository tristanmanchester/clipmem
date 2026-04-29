import AppKit
import SwiftUI

struct MenuBarPanelView: View {
    let appModel: AppModel

    @Environment(\.openWindow) private var openWindow
    @Environment(\.openSettings) private var openSettings
    @State private var recentSearchQuery = ""
    @State private var restoringItemID: Int?
    @FocusState private var searchFocused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            healthBanner
                .padding([.horizontal, .top])

            if appModel.updateStatus.isUpdateAvailable {
                UpdateBanner(
                    status: appModel.updateStatus,
                    onCopyCommand: { appModel.copyUpgradeCommand() },
                    onOpenRelease: { appModel.openUpdateRelease() }
                )
                .padding([.horizontal, .top])
            }

            recentsSearchField
                .padding([.horizontal, .top])
                .padding(.bottom, Spacing.sm)

            recentsContent

            Divider()

            footer
                .padding(Spacing.md)
        }
        .overlay(alignment: .top) {
            ActionFeedbackOverlay(message: appModel.actionMessage)
                .padding(.top, Spacing.sm)
        }
        .animation(DesignAnimation.standard, value: appModel.healthState)
        .animation(DesignAnimation.standard, value: appModel.updateStatus.isUpdateAvailable)
        .onAppear {
            searchFocused = true
            Task {
                await appModel.refreshRecentPreviewIfStale(maxAge: 1)
            }
        }
    }

    // MARK: - Health Banner

    @ViewBuilder
    private var healthBanner: some View {
        let state = appModel.healthState
        HealthBanner(
            state: state,
            errorDetail: appModel.lastError,
            isRunningAction: appModel.isRunningAction,
            actionLabel: healthActionLabel(for: state),
            onAction: { healthAction(for: state) }
        )
    }

    private func healthActionLabel(for state: HealthState) -> String? {
        switch state {
        case .setupNeeded: "Run Setup"
        case .missingBinary: "Open Settings"
        case .watcherStopped: "Start"
        case .conflict, .error: "Diagnostics"
        case .capturePaused: "Resume"
        case .stale, .noRecentCaptures: "Refresh"
        case .healthy, .unknown: nil
        }
    }

    private func healthAction(for state: HealthState) {
        switch state {
        case .setupNeeded:
            Task { await appModel.runSetup() }
        case .missingBinary:
            WindowActivation.openSettings(openSettings)
        case .watcherStopped:
            Task { await appModel.serviceAction("start") }
        case .conflict, .error:
            appModel.requestSettingsTab(.diagnostics)
            WindowActivation.openSettings(openSettings)
        case .capturePaused:
            Task { await appModel.runAction(.settingsPause(false), successMessage: "Capture resumed") }
        case .stale, .noRecentCaptures:
            Task { await appModel.refreshAll() }
        case .healthy, .unknown:
            break
        }
    }

    // MARK: - Search

    private var recentsSearchField: some View {
        HStack(spacing: Spacing.sm) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(.tertiary)
                .font(.system(size: DesignIcon.small))
                .padding(.leading, 2)
            TextField("Filter recent clips\u{2026}", text: $recentSearchQuery)
                .textFieldStyle(.plain)
                .focused($searchFocused)
            if !recentSearchQuery.isEmpty {
                Button {
                    recentSearchQuery = ""
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundStyle(.tertiary)
                }
                .buttonStyle(.borderless)
                .frame(minWidth: 28, minHeight: 28)
                .contentShape(Rectangle())
            }
        }
        .padding(.horizontal, Spacing.md)
        .padding(.vertical, Spacing.sm)
        .background(.quaternary, in: Capsule())
        .controlSize(.small)
    }

    // MARK: - Clipboard Items

    @ViewBuilder
    private var recentsContent: some View {
        if appModel.isRefreshing && appModel.recentPreview.isEmpty {
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if appModel.recentPreview.isEmpty && !appModel.isRefreshing {
            EmptyStateView(
                title: "Start copying",
                detail: "Items appear here automatically. Check Diagnostics for agent context when capture looks stale.",
                symbol: "clipboard",
                compact: true
            )
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if filteredRecentPreview.isEmpty && recentSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false {
            VStack(spacing: Spacing.md) {
                EmptyStateView(
                    title: "No matching recents",
                    detail: "Search the full archive in History.",
                    symbol: "magnifyingglass",
                    compact: true
                )
                Button("Open History Search", systemImage: "arrow.up.right.square") {
                    appModel.requestHistorySearch(query: recentSearchQuery)
                    WindowActivation.openWindow(openWindow, id: .history)
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            ScrollView(.vertical) {
                VStack(spacing: 0) {
                    ForEach(filteredRecentPreview, id: \.id) { item in
                        recentRow(for: item)
                    }
                }
                .padding(.vertical, Spacing.xs)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .scrollIndicators(.visible)
            .disablesScrollElasticity()
            .transaction { transaction in
                transaction.animation = nil
            }
        }
    }

    private func recentRow(for item: ClipmemItem) -> some View {
        Button {
            restoringItemID = item.snapshotId
            Task {
                await appModel.restore(item)
                try? await Task.sleep(for: .milliseconds(200))
                restoringItemID = nil
                NSApp.deactivate()
            }
        } label: {
            ResultRowView(
                item: item,
                selected: item.snapshotId == restoringItemID,
                animatedHighlight: false
            )
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .contextMenu {
            Button("Copy Plain Text") {
                if let text = item.copyablePlainText {
                    PasteboardActions.copyPlainText(text)
                }
            }
            .disabled(item.copyablePlainText == nil)
            Button("Open in History") {
                appModel.requestHistoryFocus(
                    snapshotID: item.snapshotId,
                    mode: .recent,
                    query: ""
                )
                WindowActivation.openWindow(openWindow, id: .history)
            }
            Button("Forget", role: .destructive) {
                Task { await appModel.forget(item) }
            }
        }
    }

    // MARK: - Footer

    private var footer: some View {
        HStack(spacing: Spacing.md) {
            Button {
                WindowActivation.openWindow(openWindow, id: .history)
            } label: {
                Label("History", systemImage: "clock.arrow.circlepath")
            }
            .help("Open History (\u{2318}\u{21E7}H)")
            .symbolEffect(.bounce, value: appModel.clipboardHistoryRevision)

            Button {
                WindowActivation.openWindow(openWindow, id: .quickRecall)
            } label: {
                Label("Search", systemImage: "magnifyingglass")
            }
            .help("Open Search (\u{2325}\u{21E7}V)")

            Spacer()

            Divider()
                .frame(height: 16)

            Button {
                WindowActivation.openSettings(openSettings)
            } label: {
                Label("Settings", systemImage: "gearshape")
                    .labelStyle(.iconOnly)
            }
            .frame(minWidth: 32, minHeight: 32)
            .contentShape(Rectangle())
            .help("Open Settings")

            Button {
                NSApp.terminate(nil)
            } label: {
                Label("Quit", systemImage: "power")
                    .labelStyle(.iconOnly)
            }
            .frame(minWidth: 32, minHeight: 32)
            .contentShape(Rectangle())
            .help("Quit Clipmem")
        }
        .buttonStyle(.borderless)
    }

    // MARK: - Filtering

    private var filteredRecentPreview: [ClipmemItem] {
        let query = recentSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
        guard query.isEmpty == false else { return appModel.recentPreview }
        return appModel.recentPreview.filter { item in
            searchableText(for: item).localizedCaseInsensitiveContains(query)
        }
    }

    private func searchableText(for item: ClipmemItem) -> String {
        [
            item.displayText,
            item.appName,
            item.appBundleId,
            item.kind.rawValue,
            DisplayFormatters.localTimestamp(item.observedAt),
            item.observedAt,
            item.urls?.joined(separator: " "),
            item.filePaths?.joined(separator: " "),
        ]
        .compactMap { $0 }
        .joined(separator: " ")
    }
}
