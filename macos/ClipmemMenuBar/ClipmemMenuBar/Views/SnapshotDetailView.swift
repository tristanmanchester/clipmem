import AppKit
import SwiftUI

struct SnapshotDetailView: View {
    let detail: SnapshotDetails?
    let fallback: ClipmemItem?
    let appModel: AppModel
    var isLoading: Bool = false

    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var visibleSections = 0
    @State private var imagePreviewState: ImagePreviewState = .notAvailable

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Spacing.xl) {
                if let detail {
                    if visibleSections >= 1 {
                        textSection(detail)
                            .transition(.opacity)
                    }
                    if visibleSections >= 2 {
                        Divider()
                        metadataSection(detail)
                            .transition(.opacity)
                    }
                    if visibleSections >= 3 {
                        Divider()
                        representationsSection(detail)
                            .transition(.opacity)
                    }
                    if visibleSections >= 4 {
                        Divider()
                        eventsSection(detail)
                            .transition(.opacity)
                    }
                } else if let fallback {
                    Text(fallback.displayText)
                        .textSelection(.enabled)
                        .font(DesignType.bodyPrimary)
                    Text("Select an item to load full snapshot detail.")
                        .foregroundStyle(.secondary)
                } else if isLoading {
                    loadingSkeleton
                } else {
                    EmptyStateView(title: "No Selection", detail: "Select a clipboard item to inspect it, or use Diagnostics for agent context and setup commands.", symbol: "sidebar.right")
                }
            }
            .padding()
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .overlay {
            if isLoading && detail == nil && fallback != nil {
                loadingSkeleton
                    .padding()
            }
        }
        .onChange(of: detail?.snapshotId) {
            revealSections()
        }
        .task(id: detail?.snapshotId) {
            revealSections()
            await loadImagePreview()
        }
        .onDisappear {
            removeLoadedPreview()
            imagePreviewState = .notAvailable
        }
    }

    private func revealSections() {
        if reduceMotion || detail == nil {
            visibleSections = 4
            return
        }
        visibleSections = 0
        Task { @MainActor in
            for section in 1...4 {
                try? await Task.sleep(for: .milliseconds(100))
                withAnimation(DesignAnimation.standard) {
                    visibleSections = section
                }
            }
        }
    }

    @ViewBuilder
    private func textSection(_ detail: SnapshotDetails) -> some View {
        VStack(alignment: .leading, spacing: Spacing.sm) {
            HStack {
                Text("Content")
                    .font(DesignType.sectionHeader)
                Spacer()
                if canCopy(detail) {
                    Button(copyButtonTitle(for: detail), systemImage: "doc.on.doc") {
                        copy(detail)
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    .help(copyButtonHelp(for: detail))
                }
            }

            if detail.imagePreviewRepresentation != nil {
                imagePreviewView
            }

            if let text = contentText(from: detail) {
                CommandClickableMarkdownText(
                    rendered: MarkdownTextRenderer.renderedText(text, style: .detail),
                    lineLimit: nil,
                    truncationMode: .tail,
                    selectionEnabled: true
                )
                    .font(DesignType.bodyPrimary)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(Spacing.md)
                    .background(Color(.textBackgroundColor), in: .rect(cornerRadius: DesignRadius.md))
                    .shadow(color: .black.opacity(0.04), radius: 2, y: 1)
            } else {
                ContentUnavailableView("No Extracted Text", systemImage: "shippingbox", description: Text("This snapshot appears to be binary, image, PDF, or otherwise has no extracted text. Metadata and export actions are available."))
            }
        }
    }

    @ViewBuilder
    private var imagePreviewView: some View {
        switch imagePreviewState {
        case .notAvailable:
            EmptyView()
        case .loading:
            ProgressView()
                .frame(maxWidth: .infinity, minHeight: 180)
                .background(Color(.textBackgroundColor), in: .rect(cornerRadius: DesignRadius.md))
        case .loaded(_, _, let image):
            Image(nsImage: image)
                .resizable()
                .scaledToFit()
                .frame(maxWidth: .infinity, maxHeight: 380)
                .padding(Spacing.sm)
                .background(Color(.textBackgroundColor), in: .rect(cornerRadius: DesignRadius.md))
                .shadow(color: .black.opacity(0.04), radius: 2, y: 1)
        case .failed(_, let message):
            ContentUnavailableView("Preview Unavailable", systemImage: "photo", description: Text(message))
                .frame(maxWidth: .infinity, minHeight: 160)
                .background(Color(.textBackgroundColor), in: .rect(cornerRadius: DesignRadius.md))
        }
    }

    private func contentText(from detail: SnapshotDetails) -> String? {
        if detail.snapshotKind == .image, let ocrText = detail.ocrText, ocrText.isEmpty == false {
            return ocrText
        }
        if detail.shouldHideImagePlaceholderText, detail.imagePreviewRepresentation != nil {
            return nil
        }
        return bestText(from: detail)
    }

    private func bestText(from detail: SnapshotDetails) -> String? {
        [detail.bestText, detail.previewText, detail.textSummary, detail.ocrText]
            .compactMap { $0 }
            .first(where: { $0.isEmpty == false })
    }

    private func canCopy(_ detail: SnapshotDetails) -> Bool {
        if detail.copyableDetailText?.isEmpty == false {
            return true
        }
        return detail.itemCount > 0
    }

    private func copyButtonTitle(for detail: SnapshotDetails) -> String {
        if detail.copyableDetailText?.isEmpty == false {
            return "Copy"
        }
        if detail.snapshotKind == .image {
            return "Copy Image"
        }
        return "Copy Snapshot"
    }

    private func copyButtonHelp(for detail: SnapshotDetails) -> String {
        if detail.copyableDetailText?.isEmpty == false {
            return "Copy extracted text"
        }
        return "Copy this saved clipboard item with its original formats"
    }

    private func copy(_ detail: SnapshotDetails) {
        if let text = detail.copyableDetailText {
            appModel.copyPlainTextToPasteboard(text)
            return
        }
        Task { await appModel.copySnapshotToPasteboard(snapshotID: detail.snapshotId) }
    }

    private func loadImagePreview() async {
        guard let detail, let representation = detail.imagePreviewRepresentation else {
            removeLoadedPreview()
            imagePreviewState = .notAvailable
            return
        }

        let snapshotID = detail.snapshotId
        removeLoadedPreview()
        imagePreviewState = .loading(snapshotID: snapshotID)

        let destination = FileManager.default.temporaryDirectory
            .appendingPathComponent("clipmem-preview-\(snapshotID)-\(representation.itemIndex)-\(UUID().uuidString)")
            .appendingPathExtension(representation.fileExtension)

        do {
            _ = try await appModel.client.export(
                snapshotID: snapshotID,
                itemIndex: representation.itemIndex,
                uti: representation.uti,
                destination: destination.path,
                force: true
            )
            guard self.detail?.snapshotId == snapshotID else {
                try? FileManager.default.removeItem(at: destination)
                return
            }
            guard let image = NSImage(contentsOf: destination) else {
                try? FileManager.default.removeItem(at: destination)
                imagePreviewState = .failed(snapshotID: snapshotID, message: "The saved image data could not be decoded.")
                return
            }
            imagePreviewState = .loaded(snapshotID: snapshotID, url: destination, image: image)
        } catch is CancellationError {
            try? FileManager.default.removeItem(at: destination)
        } catch {
            guard self.detail?.snapshotId == snapshotID else {
                try? FileManager.default.removeItem(at: destination)
                return
            }
            imagePreviewState = .failed(snapshotID: snapshotID, message: error.localizedDescription)
        }
    }

    private func removeLoadedPreview() {
        if case .loaded(_, let url, _) = imagePreviewState {
            try? FileManager.default.removeItem(at: url)
        }
    }

    private func metadataSection(_ detail: SnapshotDetails) -> some View {
        GroupBox("Metadata") {
            Grid(alignment: .leading, horizontalSpacing: Spacing.md, verticalSpacing: Spacing.sm) {
                FieldRow(title: "Kind", value: detail.snapshotKind.displayTitle)
                FieldRow(title: "Snapshot ID", value: String(detail.snapshotId))
                FieldRow(title: "Content fingerprint", value: detail.sha256, lineLimit: 1)
                FieldRow(title: "First Seen", value: DisplayFormatters.localTimestamp(detail.firstObservedAt))
                FieldRow(title: "Last Seen", value: DisplayFormatters.localTimestamp(detail.lastObservedAt))
                FieldRow(title: "Capture Count", value: String(detail.captureCount))
                FieldRow(title: "Bytes", value: DisplayFormatters.byteCount(detail.totalBytes) ?? String(detail.totalBytes))
                FieldRow(title: "OCR status", value: detail.ocrStatus)
                FieldRow(title: "App Hint", value: detail.lastFrontmostAppName.map { "Copied while in \($0)" })
                FieldRow(title: "App identifier", value: detail.lastFrontmostAppBundleId, lineLimit: 1)
                FieldRow(title: "URLs", value: detail.urls.joined(separator: "\n"), lineLimit: 3)
                FieldRow(title: "Files", value: detail.filePaths.joined(separator: "\n"), lineLimit: 3)
            }
        }
    }

    private func representationsSection(_ detail: SnapshotDetails) -> some View {
        GroupBox("Data Formats") {
            VStack(alignment: .leading, spacing: Spacing.md) {
                ForEach(detail.items) { item in
                    VStack(alignment: .leading, spacing: Spacing.xs) {
                        Text("Item \(item.itemIndex)")
                            .font(.subheadline.weight(.semibold))
                        ForEach(item.representations) { representation in
                            HStack {
                                Text(humanReadableType(representation.uti))
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                    .help(representation.uti)
                                Text(representation.kind.rawValue)
                                    .lineLimit(1)
                                Text("\(representation.byteLen) bytes")
                                    .monospacedDigit()
                                    .lineLimit(1)
                            }
                            .font(DesignType.rowMeta)
                            .foregroundStyle(.secondary)
                        }
                    }
                }
            }
        }
    }

    private func eventsSection(_ detail: SnapshotDetails) -> some View {
        GroupBox("Recent Events") {
            VStack(alignment: .leading, spacing: Spacing.sm) {
                ForEach(detail.recentEvents) { event in
                    HStack {
                        Text("#\(event.eventId)")
                            .monospacedDigit()
                        Text(DisplayFormatters.relativeTimestamp(event.observedAt) ?? event.observedAt)
                            .help(DisplayFormatters.localTimestamp(event.observedAt) ?? event.observedAt)
                        if let app = event.frontmostAppName {
                            Text("Copied while in \(app)")
                                .lineLimit(1)
                                .truncationMode(.tail)
                        }
                    }
                    .font(DesignType.rowMeta)
                    .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var loadingSkeleton: some View {
        VStack(alignment: .leading, spacing: Spacing.xl) {
            VStack(alignment: .leading, spacing: Spacing.sm) {
                Text("Content")
                    .font(DesignType.sectionHeader)
                RoundedRectangle(cornerRadius: DesignRadius.sm)
                    .fill(.quaternary)
                    .frame(height: 80)
            }
            Divider()
            VStack(alignment: .leading, spacing: Spacing.sm) {
                Text("Metadata")
                    .font(DesignType.sectionHeader)
                ForEach(0..<4, id: \.self) { _ in
                    RoundedRectangle(cornerRadius: DesignRadius.sm)
                        .fill(.quaternary)
                        .frame(height: 16)
                        .frame(maxWidth: 300)
                }
            }
        }
        .redacted(reason: .placeholder)
    }
}

private enum ImagePreviewState {
    case notAvailable
    case loading(snapshotID: Int)
    case loaded(snapshotID: Int, url: URL, image: NSImage)
    case failed(snapshotID: Int, message: String)
}
