import SwiftUI

struct QuickRecallWindowView: View {
    let appModel: AppModel

    @Environment(\.dismiss) private var dismiss
    @Environment(\.openWindow) private var openWindow
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @FocusState private var queryFocused: Bool
    @State private var quick: QuickRecallModel
    @State private var confirmForget = false
    @State private var pendingForgetItem: ClipmemItem?
    @State private var displayMode: DisplayMode = .search
    @State private var searchStyle: SearchStyle = .smart

    init(appModel: AppModel) {
        self.appModel = appModel
        _quick = State(initialValue: QuickRecallModel(appModel: appModel))
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            list
            if let error = quick.error {
                ErrorBanner(
                    message: error.message,
                    recovery: error.recovery,
                    onRetry: { Task { await quick.refresh() } }
                )
                .padding()
            }
            Divider()
            footer
        }
        .overlay(alignment: .top) {
            ActionFeedbackOverlay(message: appModel.actionMessage)
                .padding(.top, Spacing.sm)
        }
        .task {
            queryFocused = true
            syncMode()
            await quick.refresh()
        }
        .onMoveCommand { direction in
            switch direction {
            case .down: quick.moveSelection(1)
            case .up: quick.moveSelection(-1)
            default: break
            }
        }
        .onExitCommand {
            dismiss()
        }
        .onKeyPress(.space) {
            guard quick.selectedItem != nil, queryFocused == false else { return .ignored }
            openHistory()
            return .handled
        }
        .confirmationDialog("Forget this snapshot?", isPresented: $confirmForget) {
            Button("Forget", role: .destructive) {
                let item = pendingForgetItem
                Task {
                    if let item {
                        await quick.forget(item)
                    }
                    pendingForgetItem = nil
                }
            }
            Button("Cancel", role: .cancel) {
                pendingForgetItem = nil
            }
        } message: {
            Text("This permanently removes the saved content and all records of when it was copied. This cannot be undone.")
        }
        .onChange(of: confirmForget) {
            if confirmForget == false {
                pendingForgetItem = nil
            }
        }
        .onChange(of: appModel.clipboardHistoryRevision) {
            Task {
                syncMode()
                await quick.refresh()
            }
        }
    }

    // MARK: - Header

    private var header: some View {
        VStack(spacing: Spacing.md) {
            HStack(spacing: Spacing.md) {
                Picker("Mode", selection: $displayMode) {
                    ForEach(DisplayMode.allCases) { mode in
                        Text(mode.title).tag(mode)
                    }
                }
                .pickerStyle(.segmented)
                .fixedSize()
                .onChange(of: displayMode) {
                    syncMode()
                    Task { await quick.refresh() }
                }

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
                        Task { await quick.refresh() }
                    }
                }

                Spacer()
            }

            HStack(spacing: Spacing.sm) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(.tertiary)
                TextField(searchPrompt, text: $quick.query)
                    .textFieldStyle(.plain)
                    .font(.title3)
                    .focused($queryFocused)
                    .disabled(displayMode == .recent || displayMode == .timeline)
                    .onSubmit {
                        Task { await quick.restoreSelected() }
                    }
                    .onChange(of: quick.query) {
                        quick.queryChanged()
                    }
                if !quick.query.isEmpty {
                    Button {
                        withAnimation(DesignAnimation.quick) {
                            quick.query = ""
                        }
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundStyle(.tertiary)
                    }
                    .buttonStyle(.borderless)
                    .frame(minWidth: 28, minHeight: 28)
                    .contentShape(Rectangle())
                    .transition(.scale(scale: 0.25).combined(with: .opacity))
                }
            }
            .padding(.horizontal, Spacing.md)
            .padding(.vertical, Spacing.sm)
            .background(.quaternary, in: .rect(cornerRadius: DesignRadius.md))

            if !quick.results.isEmpty {
                Text("\(quick.results.count) result\(quick.results.count == 1 ? "" : "s")")
                    .font(DesignType.rowMeta)
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .transition(.opacity)
            }
        }
        .padding()
    }

    private var searchPrompt: String {
        switch displayMode {
        case .search:
            searchStyle == .smart ? "Describe what you're looking for\u{2026}" : "Search for exact text\u{2026}"
        case .recent:
            "Recent mode uses filters"
        case .timeline:
            "Timeline mode uses filters"
        }
    }

    // MARK: - List

    private var list: some View {
        List(selection: $quick.selectedID) {
            ForEach(Array(quick.results.enumerated()), id: \.element.id) { index, item in
                ResultRowView(item: item, selected: item.snapshotId == quick.selectedID)
                    .tag(item.snapshotId)
                    .transition(.opacity.combined(with: .scale(scale: 0.97, anchor: .top)))
                    .animation(DesignAnimation.staggerDelay(index: index, reduceMotion: reduceMotion), value: quick.results.count)
                    .contextMenu {
                        Button("Restore") { Task { await appModel.restore(item) } }
                        Button("Open in History") { openHistory(item: item) }
                        Button("Forget", role: .destructive) {
                            pendingForgetItem = item
                            confirmForget = true
                        }
                    }
            }
        }
        .listStyle(.inset)
        .overlay {
            if quick.isLoading {
                ProgressView()
                    .transition(.opacity)
            } else if quick.results.isEmpty {
                EmptyStateView(
                    title: "No matches found",
                    detail: displayMode == .search
                        ? "Try different keywords, switch to \(searchStyle == .smart ? "Exact" : "Smart") mode, or check agent context in Diagnostics."
                        : "Try another mode or check agent context in Diagnostics.",
                    symbol: "magnifyingglass",
                    compact: true
                )
            }
        }
    }

    // MARK: - Footer

    private var footer: some View {
        HStack(spacing: Spacing.lg) {
            VStack(spacing: Spacing.xxs) {
                Button("Restore", systemImage: "arrow.uturn.backward.square") {
                    Task { await quick.restoreSelected() }
                }
                .buttonStyle(.borderedProminent)
                .keyboardShortcut(.return, modifiers: [])
                .disabled(quick.selectedItem == nil)
                Text("Return")
                    .font(.system(size: 9))
                    .foregroundStyle(.tertiary)
            }
            .pressable()

            VStack(spacing: Spacing.xxs) {
                Button("Open in History", systemImage: "rectangle.stack.badge.play") {
                    openHistory()
                }
                .buttonStyle(.bordered)
                .keyboardShortcut("o", modifiers: .command)
                .disabled(quick.selectedItem == nil)
                Text("\u{2318}O")
                    .font(.system(size: 9))
                    .foregroundStyle(.tertiary)
            }

            Spacer()

            VStack(spacing: Spacing.xxs) {
                Button("Forget", systemImage: "trash", role: .destructive) {
                    pendingForgetItem = quick.selectedItem
                    confirmForget = true
                }
                .buttonStyle(.borderless)
                .foregroundStyle(.red)
                .keyboardShortcut(.delete, modifiers: [])
                .disabled(quick.selectedItem == nil)
                Text("Delete")
                    .font(.system(size: 9))
                    .foregroundStyle(.tertiary)
            }
        }
        .padding()
    }

    // MARK: - Helpers

    private func syncMode() {
        quick.mode = displayMode.queryMode(searchStyle: searchStyle)
    }

    private func openHistory() {
        guard let item = quick.selectedItem else { return }
        openHistory(item: item)
    }

    private func openHistory(item: ClipmemItem) {
        appModel.requestHistoryFocus(snapshotID: item.snapshotId, mode: quick.mode, query: quick.query)
        WindowActivation.openWindow(openWindow, id: .history)
    }
}
