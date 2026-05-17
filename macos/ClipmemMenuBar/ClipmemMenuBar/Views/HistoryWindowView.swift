import SwiftUI

struct HistoryWindowView: View {
    let appModel: AppModel

    @Environment(\.openWindow) private var openWindow
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var history: HistoryModel
    @SceneStorage("history.mode") private var storedMode = ""
    @SceneStorage("history.query") private var storedQuery = ""
    @SceneStorage("history.inspector") private var inspectorPresented = false
    @SceneStorage("history.selected") private var storedSelectedID = 0
    @State private var handledHistoryOpenRequestID = 0
    @State private var displayMode: DisplayMode = .recent
    @State private var searchStyle: SearchStyle = .smart

    init(appModel: AppModel) {
        self.appModel = appModel
        _history = State(initialValue: HistoryModel(appModel: appModel))
    }

    var body: some View {
        NavigationSplitView {
            sidebar
        } content: {
            contentColumn
        } detail: {
            detailColumn
        }
        .navigationTitle(displayMode.title)
        .toolbar {
            ToolbarItem(placement: .principal) {
                HStack(spacing: Spacing.sm) {
                    Text(displayMode.title)
                        .font(DesignType.bodySecondary.weight(.medium))
                    if !history.results.isEmpty {
                        Text("\u{2014} \(history.results.count) item\(history.results.count == 1 ? "" : "s")")
                            .font(DesignType.bodySecondary)
                            .monospacedDigit()
                            .foregroundStyle(.secondary)
                    }
                }
            }
            ToolbarItemGroup {
                Button("Refresh", systemImage: "arrow.clockwise") {
                    Task { await history.reload() }
                }
                .keyboardShortcut("r", modifiers: .command)
                Button("Search", systemImage: "magnifyingglass") {
                    WindowActivation.openWindow(openWindow, id: .quickRecall)
                }
                Button("Inspector", systemImage: inspectorPresented ? "sidebar.right.fill" : "sidebar.right") {
                    inspectorPresented.toggle()
                }
                .help("Toggle inspector (\u{2318}\u{21E7}I)")
            }
        }
        .inspector(isPresented: $inspectorPresented) {
            inspector
                .inspectorColumnWidth(min: 220, ideal: 260, max: 320)
        }
        .overlay(alignment: .top) {
            ActionFeedbackOverlay(message: appModel.actionMessage)
                .padding(.top, Spacing.sm)
        }
        .navigationSplitViewStyle(.balanced)
        .background {
            GeometryReader { proxy in
                Color.clear.preference(key: HistoryWindowWidthKey.self, value: proxy.size.width)
            }
        }
        .onPreferenceChange(HistoryWindowWidthKey.self) { width in
            if inspectorPresented, width < 1_400 {
                inspectorPresented = false
            }
        }
        .task {
            restoreSceneState()
            if await applyPendingHistoryOpenRequestIfNeeded() == false {
                await history.reload()
            }
        }
        .onChange(of: appModel.pendingHistoryOpenRequest?.id) {
            Task {
                await applyPendingHistoryOpenRequestIfNeeded()
            }
        }
        .onChange(of: history.query) {
            storedQuery = history.query
        }
        .onChange(of: history.selectedID) {
            storedSelectedID = history.selectedID ?? 0
            Task { await history.loadSelectedDetail() }
        }
        .onChange(of: appModel.clipboardHistoryRevision) {
            Task { await history.refreshForExternalHistoryChange() }
        }
    }

    // MARK: - Sidebar

    private var sidebar: some View {
        List(selection: displayModeBinding) {
            Section("Browse") {
                ForEach(DisplayMode.allCases) { mode in
                    Label(mode.title, systemImage: mode.symbol)
                        .tag(mode)
                }
            }
            if let status = appModel.serviceStatus {
                Section("Statistics") {
                    LabeledContent("Database", value: DisplayFormatters.byteCount(status.dbSizeBytes) ?? "\u{2014}")
                        .font(DesignType.rowMeta)
                    if let retention = status.retention {
                        LabeledContent("Retention", value: retention)
                            .font(DesignType.rowMeta)
                    }
                }
            }
        }
        .navigationTitle("clipmem")
        .navigationSplitViewColumnWidth(min: 150, ideal: 200, max: 220)
        .safeAreaInset(edge: .bottom) {
            sidebarStatusIndicator
                .padding(.horizontal, Spacing.md)
                .padding(.vertical, Spacing.sm)
        }
    }

    private var sidebarStatusIndicator: some View {
        HStack(spacing: Spacing.sm) {
            Circle()
                .fill(appModel.healthState.tint)
                .frame(width: 8, height: 8)
            Text(appModel.healthState.title)
                .font(DesignType.rowMeta)
                .foregroundStyle(.secondary)
                .lineLimit(1)
        }
    }

    private var displayModeBinding: Binding<DisplayMode> {
        Binding {
            displayMode
        } set: { newMode in
            guard displayMode != newMode else { return }
            displayMode = newMode
            syncMode()
            storedMode = history.mode.rawValue
            Task { await history.reload() }
        }
    }

    // MARK: - Content Column

    private var contentColumn: some View {
        VStack(spacing: 0) {
            queryControls
                .padding()
            Divider()
            resultList
        }
        .navigationTitle(displayMode.title)
        .navigationSplitViewColumnWidth(min: 320, ideal: 420, max: 560)
    }

    private var detailColumn: some View {
        SnapshotDetailView(
            detail: history.selectedDetail,
            fallback: history.selectedItem,
            appModel: appModel,
            isLoading: history.isLoadingDetail
        )
            .navigationTitle(displayMode.title)
            .navigationSplitViewColumnWidth(min: 360, ideal: 580)
    }

    // MARK: - Query Controls

    private var queryControls: some View {
        VStack(spacing: Spacing.md) {
            HStack(spacing: Spacing.sm) {
                if displayMode == .search {
                    Picker("Style", selection: $searchStyle) {
                        Text("Smart").tag(SearchStyle.smart)
                        Text("Exact").tag(SearchStyle.exact)
                    }
                    .pickerStyle(.segmented)
                    .fixedSize()
                    .controlSize(.small)
                    .onChange(of: searchStyle) {
                        syncMode()
                        Task { await history.reload() }
                    }
                }

                TextField(searchPrompt, text: $history.query)
                    .textFieldStyle(.roundedBorder)
                    .disabled(displayMode == .recent || displayMode == .timeline)
                    .onSubmit {
                        Task { await history.reload() }
                    }
                Button("Search", systemImage: "magnifyingglass") {
                    Task { await history.reload() }
                }
                .disabled(displayMode == .search && history.query.isEmpty)
            }
            .padding(Spacing.md)
            .background(.quaternary.opacity(0.5), in: .rect(cornerRadius: DesignRadius.md))

            FilterBar(history: history)
        }
    }

    private var searchPrompt: String {
        switch displayMode {
        case .search:
            searchStyle == .smart ? "Describe what you want to recall" : "Search for exact text"
        case .recent:
            "Recent mode uses filters"
        case .timeline:
            "Timeline mode uses filters"
        }
    }

    // MARK: - Results

    private var resultList: some View {
        VStack(spacing: 0) {
            if let error = history.error {
                ErrorBanner(
                    message: error.message,
                    recovery: error.recovery,
                    onRetry: { Task { await history.reload() } }
                )
                .padding()
            }
            List(selection: $history.selectedID) {
                ForEach(Array(history.results.enumerated()), id: \.element.id) { index, item in
                    ResultRowView(item: item, selected: item.snapshotId == history.selectedID)
                        .tag(item.snapshotId)
                        .animation(DesignAnimation.staggerDelay(index: index, reduceMotion: reduceMotion), value: history.results.count)
                        .onAppear {
                            if item.snapshotId == history.results.last?.snapshotId,
                               history.nextCursor != nil {
                                Task { await history.loadMore() }
                            }
                        }
                }
                if history.nextCursor != nil {
                    Button("Load More", systemImage: "arrow.down.circle") {
                        Task { await history.loadMore() }
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, Spacing.sm)
                }
            }
            .listStyle(.inset)
            .overlay {
                if !history.isLoading && history.results.isEmpty && history.error == nil {
                    EmptyStateView(
                        title: displayMode == .recent || displayMode == .timeline ? "No recent history" : "No results",
                        detail: displayMode == .recent || displayMode == .timeline
                            ? "Start copying to build your clipboard history, or run clipmem agents context --format json to check capture health."
                            : "Try adjusting your filters or use the agent context command in Diagnostics to check archive freshness.",
                        symbol: displayMode == .recent || displayMode == .timeline ? "clock" : "magnifyingglass"
                    )
                }
            }
            if history.isLoading {
                ProgressView()
                    .padding(Spacing.sm)
            }
        }
    }

    // MARK: - Inspector

    private var inspector: some View {
        VStack(alignment: .leading, spacing: Spacing.lg) {
            Text("Inspector")
                .font(DesignType.sectionHeader)
            if let selected = history.selectedItem {
                Text(selected.displayText)
                    .font(DesignType.bodySecondary)
                    .lineLimit(2)
                    .foregroundStyle(.secondary)

                Divider()

                Text("Metadata")
                    .font(DesignType.rowMeta.weight(.semibold))
                    .foregroundStyle(.secondary)
                    .textCase(.uppercase)
                Grid(alignment: .leading, horizontalSpacing: Spacing.md, verticalSpacing: Spacing.sm) {
                    FieldRow(title: "Snapshot", value: String(selected.snapshotId))
                    FieldRow(title: "Event", value: selected.eventId.map(String.init))
                    FieldRow(title: "Kind", value: selected.kind.displayTitle)
                    FieldRow(title: "Bytes", value: selected.totalBytes.map(String.init))
                    FieldRow(title: "Matched", value: selected.matchedFields?.joined(separator: ", "))
                    FieldRow(title: "Why", value: selected.whyMatched)
                }

                Divider()

                Text("Actions")
                    .font(DesignType.rowMeta.weight(.semibold))
                    .foregroundStyle(.secondary)
                    .textCase(.uppercase)
                ItemActionButtons(
                    item: selected,
                    detail: history.selectedDetail,
                    appModel: appModel,
                    onForgot: { await history.forgetSelected() }
                )
            } else {
                Text("Select a result for metadata and actions.")
                    .foregroundStyle(.secondary)
            }
            Spacer()
        }
        .padding()
        .background(.ultraThinMaterial)
    }

    // MARK: - State Management

    private func syncMode() {
        history.mode = displayMode.queryMode(searchStyle: searchStyle)
        storedMode = history.mode.rawValue
    }

    private func restoreSceneState() {
        let restoredMode = storedMode.isEmpty ? UserDefaults.standard.clipmemDefaultMode.rawValue : storedMode
        let queryMode = (QueryMode(rawValue: restoredMode) ?? .recent).historyCompatibleMode
        let (dm, ss) = DisplayMode.from(queryMode: queryMode)
        displayMode = dm
        searchStyle = ss
        history.mode = queryMode
        storedMode = queryMode.rawValue
        history.query = storedQuery
        history.selectedID = storedSelectedID == 0 ? nil : storedSelectedID
    }

    @discardableResult
    private func applyPendingHistoryOpenRequestIfNeeded() async -> Bool {
        guard let request = appModel.pendingHistoryOpenRequest else { return false }
        guard request.id != handledHistoryOpenRequestID else { return false }

        handledHistoryOpenRequestID = request.id

        let queryMode = request.mode.historyCompatibleMode
        let (dm, ss) = DisplayMode.from(queryMode: queryMode)
        displayMode = dm
        searchStyle = ss
        history.mode = queryMode

        history.query = request.query
        storedMode = history.mode.rawValue
        storedQuery = request.query
        storedSelectedID = request.focusedSnapshotID ?? 0
        await history.reload(selecting: request.focusedSnapshotID)
        return true
    }
}

private struct HistoryWindowWidthKey: SwiftUI.PreferenceKey {
    static let defaultValue: CGFloat = 0

    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = nextValue()
    }
}
