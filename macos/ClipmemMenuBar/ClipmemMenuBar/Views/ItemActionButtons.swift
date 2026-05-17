import AppKit
import SwiftUI

struct ItemActionButtons: View {
    let item: ClipmemItem?
    let detail: SnapshotDetails?
    let appModel: AppModel
    var onForgot: (() async -> Void)?

    @State private var confirmForget = false

    var body: some View {
        VStack(alignment: .leading, spacing: Spacing.sm) {
            Button("Restore Snapshot", systemImage: "arrow.uturn.backward.square") {
                guard let item else { return }
                Task { await appModel.restore(item) }
            }
            .disabled(item == nil)
            .keyboardShortcut(.return, modifiers: .command)

            Button("Copy Plain Text", systemImage: "doc.on.doc") {
                let text = detail?.bestText ?? item?.bestText ?? ""
                appModel.copyPlainTextToPasteboard(text)
            }
            .disabled((detail?.bestText ?? item?.bestText ?? "").isEmpty)
            .keyboardShortcut("c", modifiers: [.command, .shift])

            Button("Open URL", systemImage: "safari") {
                PasteboardActions.openSingleURL(detail?.urls ?? item?.urls)
            }
            .disabled((detail?.urls ?? item?.urls ?? []).count != 1)

            Button("Reveal File", systemImage: "finder") {
                PasteboardActions.revealFilePath(detail?.filePaths ?? item?.filePaths)
            }
            .disabled((detail?.filePaths ?? item?.filePaths ?? []).isEmpty)

            Menu("Export Representation", systemImage: "square.and.arrow.down") {
                if let detail {
                    ForEach(detail.items) { clipboardItem in
                        ForEach(clipboardItem.representations) { representation in
                            Button("\(clipboardItem.itemIndex): \(humanReadableType(representation.uti))") {
                                export(clipboardItem: clipboardItem, representation: representation)
                            }
                        }
                    }
                } else {
                    Text("Load detail first")
                }
            }
            .disabled(detail == nil)

            Divider()

            Button("Forget Snapshot", systemImage: "trash", role: .destructive) {
                confirmForget = true
            }
            .disabled(item == nil)
        }
        .buttonStyle(.bordered)
        .controlSize(.small)
        .confirmationDialog("Forget this snapshot?", isPresented: $confirmForget) {
            Button("Forget", role: .destructive) {
                Task {
                    if let onForgot {
                        await onForgot()
                    } else if let item {
                        await appModel.forget(item)
                    }
                }
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This permanently removes the saved content and all records of when it was copied. This cannot be undone.")
        }
    }

    private func export(clipboardItem: ClipboardItemDetail, representation: ClipboardRepresentation) {
        guard let detail else { return }
        let defaultName = "clipmem-\(detail.snapshotId)-\(clipboardItem.itemIndex)"
        guard let destination = ExportDestination.choose(defaultName: defaultName) else { return }
        Task {
            do {
                _ = try await appModel.client.export(
                    snapshotID: detail.snapshotId,
                    itemIndex: clipboardItem.itemIndex,
                    uti: representation.uti,
                    destination: destination,
                    force: true
                )
                appModel.lastError = nil
                appModel.actionMessage = "Exported successfully"
            } catch {
                appModel.lastError = UserError(error)
            }
        }
    }
}
