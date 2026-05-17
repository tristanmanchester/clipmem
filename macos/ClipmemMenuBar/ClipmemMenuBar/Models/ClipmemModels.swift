import Foundation
import UniformTypeIdentifiers

enum SnapshotKind: String, Decodable, Hashable, Sendable {
    case empty
    case mixed
    case plainText = "plain_text"
    case url
    case fileUrl = "file_url"
    case html
    case json
    case xml
    case rtf
    case pdf
    case image
    case binary

    var displayTitle: String {
        switch self {
        case .empty: "empty"
        case .mixed: "mixed"
        case .plainText: "plain_text"
        case .url: "url"
        case .fileUrl: "file_url"
        case .html: "html"
        case .json: "json"
        case .xml: "xml"
        case .rtf: "rtf"
        case .pdf: "pdf"
        case .image: "image"
        case .binary: "binary"
        }
    }
}

enum ClipboardRepresentationKind: String, Decodable, Hashable, Sendable {
    case plainText = "plain_text"
    case url
    case fileUrl = "file_url"
    case html
    case json
    case xml
    case rtf
    case pdf
    case image
    case binary
    case empty
}

enum ServiceProvider: String, Decodable, Equatable, Sendable {
    case homebrew
    case launchagent
}

enum ServiceState: String, Decodable, Equatable, Sendable {
    case notInstalled = "not_installed"
    case installed
    case loaded
    case running
}

enum RecallMatchConfidence: String, Decodable, Equatable, Sendable {
    case high
    case medium
    case low
}

struct ProviderStatus: Decodable, Equatable, Sendable {
    var provider: ServiceProvider
    var label: String
    var state: ServiceState
    var installed: Bool
    var loaded: Bool
    var running: Bool
    var pid: Int?
    var plistPath: String?
    var configuredBinaryPath: String?
    var runningCommand: String?
    var runningBinaryPath: String?
    var stdoutLogPath: String?
    var stderrLogPath: String?
}

struct ServiceStatusReport: Decodable, Equatable, Sendable {
    var binaryPath: String
    var dbPath: String
    var preferredProvider: ServiceProvider
    var preferredProviderReason: String
    var conflict: Bool
    var homebrew: ProviderStatus
    var launchagent: ProviderStatus
    var dbExists: Bool
    var dbSizeBytes: Int?
    var recentCaptureAt: String?
    var recentCaptureWithinLastHour: Bool?
    var paused: Bool?
    var apiKeyFilterEnabled: Bool?
    var retentionSeconds: UInt64?
    var retention: String?
    var ignoredBundleIdCount: Int?
    var revision: ArchiveRevision?
    var stale: Bool
    var dbError: String?
    var watcherBinaryMismatch: Bool
    var watcherBinaryMismatchNote: String?
    var notes: [String]

    var health: HealthState {
        if conflict { return .conflict }
        if dbError != nil { return .error }
        if !dbExists { return .setupNeeded }
        if paused == true { return .capturePaused }
        if stale { return .stale }
        if isAnyProviderRunning {
            if recentCaptureWithinLastHour == false { return .noRecentCaptures }
            return .healthy
        }
        if isAnyProviderConfigured { return .watcherStopped }
        return .setupNeeded
    }

    var logPaths: [String] {
        [homebrew.stdoutLogPath, homebrew.stderrLogPath, launchagent.stdoutLogPath, launchagent.stderrLogPath]
            .compactMap { $0 }
    }

    var watcherBinaryPath: String? {
        if launchagent.running {
            return launchagent.runningBinaryPath ?? launchagent.configuredBinaryPath
        }
        if homebrew.running {
            return homebrew.runningBinaryPath ?? homebrew.configuredBinaryPath
        }
        return launchagent.configuredBinaryPath ?? homebrew.configuredBinaryPath
    }

    private var isAnyProviderRunning: Bool {
        homebrew.running || launchagent.running
    }

    private var isAnyProviderConfigured: Bool {
        providerIsConfigured(homebrew) || providerIsConfigured(launchagent)
    }

    private func providerIsConfigured(_ provider: ProviderStatus) -> Bool {
        provider.installed || provider.loaded || provider.running
    }
}

struct ArchiveRevision: Decodable, Equatable, Sendable {
    var revision: UInt64
    var archiveContentRevision: UInt64
    var settingsRevision: UInt64
    var ocrRevision: UInt64
    var storageRevision: UInt64
    var serviceRevision: UInt64
    var appPreferencesRevision: UInt64
    var lastChangeKind: String
    var updatedAt: String
}

struct DoctorReport: Decodable, Equatable, Sendable {
    var dbPath: String?
    var sqliteVersion: String?
    var journalMode: String?
    var fts5CompileOptionPresent: Bool?
    var fts5CreateVirtualTableOk: Bool?
    var compileOptions: [String]?
}

struct SettingsReport: Decodable, Equatable, Sendable {
    var paused: Bool
    var apiKeyFilterEnabled: Bool
    var ocrEnabled: Bool
    var retentionSeconds: UInt64?
    var retention: String
    var ignoredBundleIds: [String]
}

struct ListEnvelope: Decodable, Equatable, Sendable {
    var schemaVersion: Int?
    var command: String
    var generatedAt: String?
    var appliedFilters: [String: JSONValue]?
    var truncated: Bool
    var nextCursor: String?
    var results: [ClipmemItem]
}

struct RecallEnvelope: Decodable, Equatable, Sendable {
    var schemaVersion: Int?
    var command: String
    var generatedAt: String?
    var query: String?
    var bestCandidate: ClipmemItem
    var alternatives: [ClipmemItem]
    var bestMatchConfidence: RecallMatchConfidence
    var bestMatchScore: Double?
    var whySelected: String?
    var quotedText: String?
}

struct GetEnvelope: Decodable, Equatable, Sendable {
    var schemaVersion: Int?
    var command: String
    var generatedAt: String?
    var snapshot: SnapshotDetails
}

struct ClipmemItem: Decodable, Identifiable, Hashable, Sendable {
    var snapshotId: Int
    var eventId: Int?
    var sha256: String?
    var kind: SnapshotKind
    var observedAt: String?
    var firstSeenAt: String?
    var lastSeenAt: String?
    var appName: String?
    var appBundleId: String?
    var bestText: String?
    var bestTextUti: String?
    var textFragments: [TextFragment]?
    var urls: [String]?
    var filePaths: [String]?
    var htmlText: String?
    var rtfText: String?
    var textSummary: String?
    var ocrText: String?
    var ocrStatus: String?
    var previewText: String?
    var itemCount: Int?
    var totalBytes: Int?
    var captureCount: Int?
    var score: Double?
    var whyMatched: String?
    var matchedFields: [String]?
    var snippet: String?
    var changeCount: Int?

    var id: String { "\(eventId ?? snapshotId)-\(snapshotId)" }

    var displayText: String {
        let candidate = [snippet, bestText, previewText, textSummary, ocrText].compactMap { $0 }.first { $0.isEmpty == false }
        return candidate ?? "[No extracted text]"
    }

    var appHint: String? {
        guard let appName, appName.isEmpty == false else { return nil }
        return "Copied while in \(appName)"
    }

    var hasText: Bool {
        bestText?.isEmpty == false || previewText?.isEmpty == false || textSummary?.isEmpty == false || ocrText?.isEmpty == false
    }

    var copyablePlainText: String? {
        [bestText, previewText, ocrText]
            .compactMap { $0 }
            .first { $0.isEmpty == false }
    }
}

struct TextFragment: Decodable, Hashable, Sendable {
    var itemIndex: Int
    var uti: String
    var kind: ClipboardRepresentationKind
    var text: String
}

struct SnapshotDetails: Decodable, Equatable, Sendable {
    var snapshotId: Int
    var sha256: String
    var snapshotKind: SnapshotKind
    var bestText: String?
    var bestTextUti: String?
    var textFragments: [TextFragment]?
    var urls: [String]
    var filePaths: [String]
    var htmlText: String?
    var rtfText: String?
    var textSummary: String?
    var ocrText: String?
    var ocrStatus: String?
    var previewText: String?
    var searchText: String?
    var itemCount: Int
    var totalBytes: Int
    var createdAt: String?
    var captureCount: Int
    var firstObservedAt: String?
    var lastObservedAt: String?
    var lastFrontmostAppName: String?
    var lastFrontmostAppBundleId: String?
    var recentEvents: [CaptureEvent]
    var items: [ClipboardItemDetail]

    var imagePreviewRepresentation: ImagePreviewRepresentation? {
        for item in items {
            if let representation = item.representations.first(where: { $0.isPreviewableImage }) {
                return ImagePreviewRepresentation(
                    itemIndex: item.itemIndex,
                    uti: representation.uti,
                    fileExtension: representation.previewFileExtension
                )
            }
        }
        return nil
    }

    var copyableDetailText: String? {
        guard snapshotKind != .image else { return nil }
        return [bestText, previewText, textSummary, ocrText]
            .compactMap { $0 }
            .first { $0.isEmpty == false }
    }

    var shouldHideImagePlaceholderText: Bool {
        guard snapshotKind == .image else { return false }
        guard ocrText?.isEmpty != false else { return false }
        let text = [bestText, previewText, textSummary]
            .compactMap { $0 }
            .first { $0.isEmpty == false }?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return text.hasPrefix("[image")
    }
}

struct ImagePreviewRepresentation: Equatable, Sendable {
    var itemIndex: Int
    var uti: String
    var fileExtension: String
}

struct CaptureEvent: Decodable, Equatable, Identifiable, Sendable {
    var eventId: Int
    var observedAt: String
    var changeCount: Int?
    var frontmostAppName: String?
    var frontmostAppBundleId: String?

    var id: Int { eventId }
}

struct ClipboardItemDetail: Decodable, Equatable, Identifiable, Sendable {
    var itemIndex: Int
    var primaryKind: ClipboardRepresentationKind
    var primaryUti: String?
    var previewText: String?
    var searchText: String?
    var totalBytes: Int
    var representations: [ClipboardRepresentation]

    var id: Int { itemIndex }
}

struct ClipboardRepresentation: Decodable, Equatable, Identifiable, Sendable {
    var uti: String
    var kind: ClipboardRepresentationKind
    var isText: Bool
    var byteLen: Int
    var rawSha256: String?
    var textValue: String?

    var id: String { uti }

    var isPreviewableImage: Bool {
        if kind == .image {
            return true
        }
        return UTType(uti)?.conforms(to: .image) == true
    }

    var previewFileExtension: String {
        if let preferred = UTType(uti)?.preferredFilenameExtension {
            return preferred
        }
        switch uti {
        case "public.jpeg", "public.jpg":
            return "jpg"
        case "public.tiff":
            return "tiff"
        case "com.compuserve.gif":
            return "gif"
        default:
            return "png"
        }
    }
}

struct RestoreOutput: Decodable, Equatable, Sendable {
    var snapshotId: Int
    var itemCount: Int
    var representationCount: Int
    var totalBytes: Int
}

struct ExportOutput: Decodable, Equatable, Sendable {
    var snapshotId: Int
    var itemIndex: Int
    var uti: String
    var byteCount: Int
    var rawSha256: String
    var out: String
}

struct ForgetOutput: Decodable, Equatable, Sendable {
    var snapshotId: Int
    var itemCount: Int
    var representationCount: Int
    var captureEventCount: Int
    var totalBytes: Int
}

struct PurgeOutput: Decodable, Equatable, Sendable {
    var olderThanSeconds: UInt64
    var dryRun: Bool
    var snapshotCount: Int
    var itemCount: Int
    var representationCount: Int
    var captureEventCount: Int
    var totalBytes: Int
}

struct StorageFileSizes: Decodable, Equatable, Sendable {
    var db: UInt64
    var wal: UInt64
    var shm: UInt64
}

struct StorageCheckpointReport: Decodable, Equatable, Sendable {
    var busy: Int
    var log: Int
    var checkpointed: Int
}

struct StorageCompactOutput: Decodable, Equatable, Sendable {
    var dbPath: String
    var before: StorageFileSizes
    var after: StorageFileSizes
    var totalBeforeBytes: UInt64
    var totalAfterBytes: UInt64
    var reclaimedBytes: UInt64
    var estimatedReclaimableBytes: UInt64
    var pageCount: Int
    var freelistCount: Int
    var checkpoint: StorageCheckpointReport
    var dryRun: Bool
    var completed: Bool
}

struct ImageOptimizationOutput: Decodable, Equatable, Sendable {
    var dryRun: Bool
    var format: String
    var scannedRows: Int
    var compressedRows: Int
    var skippedRows: Int
    var conflictCount: Int
    var originalBytes: Int
    var optimizedBytes: Int
    var logicalSavedBytes: Int
    var compactRun: Bool
    var compact: StorageCompactOutput?
    var compactError: String?
    var filesystemSavedBytes: UInt64
    var filesystemGrowthBytes: UInt64
    var compactRecommended: Bool
}

enum ImageOptimizationProgressEvent: Decodable, Equatable, Sendable {
    case started(totalRows: Int)
    case scanning(ImageOptimizationProgressSnapshot)
    case compacting(ImageOptimizationProgressSnapshot)
    case complete(ImageOptimizationOutput)

    fileprivate enum CodingKeys: String, CodingKey {
        case type
        case totalRows
        case scannedRows
        case compressedRows
        case skippedRows
        case conflictCount
        case report
    }

    private enum EventType: String, Decodable {
        case started
        case scanning
        case compacting
        case complete
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(EventType.self, forKey: .type) {
        case .started:
            self = .started(totalRows: try container.decode(Int.self, forKey: .totalRows))
        case .scanning:
            self = .scanning(try ImageOptimizationProgressSnapshot(container: container))
        case .compacting:
            self = .compacting(try ImageOptimizationProgressSnapshot(container: container))
        case .complete:
            self = .complete(try container.decode(ImageOptimizationOutput.self, forKey: .report))
        }
    }
}

struct ImageOptimizationProgressSnapshot: Decodable, Equatable, Sendable {
    var scannedRows: Int
    var totalRows: Int
    var compressedRows: Int
    var skippedRows: Int
    var conflictCount: Int

    init(
        scannedRows: Int,
        totalRows: Int,
        compressedRows: Int,
        skippedRows: Int,
        conflictCount: Int
    ) {
        self.scannedRows = scannedRows
        self.totalRows = totalRows
        self.compressedRows = compressedRows
        self.skippedRows = skippedRows
        self.conflictCount = conflictCount
    }

    fileprivate init(container: KeyedDecodingContainer<ImageOptimizationProgressEvent.CodingKeys>) throws {
        scannedRows = try container.decode(Int.self, forKey: .scannedRows)
        totalRows = try container.decode(Int.self, forKey: .totalRows)
        compressedRows = try container.decode(Int.self, forKey: .compressedRows)
        skippedRows = try container.decode(Int.self, forKey: .skippedRows)
        conflictCount = try container.decode(Int.self, forKey: .conflictCount)
    }
}

struct ImageOptimizationProgressState: Equatable, Sendable {
    enum Phase: Equatable, Sendable {
        case scanning
        case compacting
    }

    var phase: Phase
    var scannedRows: Int
    var totalRows: Int
    var compressedRows: Int
    var skippedRows: Int
    var conflictCount: Int

    var fractionCompleted: Double? {
        guard phase == .scanning, totalRows > 0 else { return nil }
        return min(1, max(0, Double(scannedRows) / Double(totalRows)))
    }

    var statusText: String {
        switch phase {
        case .scanning:
            if totalRows > 0 {
                "Scanned \(scannedRows) of \(totalRows) images"
            } else {
                "Looking for images to optimize"
            }
        case .compacting:
            "Compacting database..."
        }
    }

    var detailText: String {
        "Compressed \(compressedRows) · Skipped \(skippedRows) · Conflicts \(conflictCount)"
    }
}

enum JSONValue: Decodable, Equatable, Hashable, Sendable {
    case string(String)
    case int(Int)
    case double(Double)
    case bool(Bool)
    case object([String: JSONValue])
    case array([JSONValue])
    case null

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let value = try? container.decode(Bool.self) {
            self = .bool(value)
        } else if let value = try? container.decode(Int.self) {
            self = .int(value)
        } else if let value = try? container.decode(Double.self) {
            self = .double(value)
        } else if let value = try? container.decode(String.self) {
            self = .string(value)
        } else if let value = try? container.decode([String: JSONValue].self) {
            self = .object(value)
        } else {
            self = .array(try container.decode([JSONValue].self))
        }
    }
}
