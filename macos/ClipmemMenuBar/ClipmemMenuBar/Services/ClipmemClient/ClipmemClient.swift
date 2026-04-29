import Foundation

enum ClipmemClientError: Error, LocalizedError, Equatable, Sendable {
    case binaryNotFound([String])
    case invalidArguments(String)
    case notFound(String)
    case unsupportedFormat(String)
    case setupNeeded(String)
    case platformError(String)
    case commandFailed(Int32, String)
    case decodingFailed(String)

    var errorDescription: String? {
        switch self {
        case .binaryNotFound:
            "clipmem binary was not found."
        case .invalidArguments(let message),
             .notFound(let message),
             .unsupportedFormat(let message),
             .setupNeeded(let message),
             .platformError(let message),
             .commandFailed(_, let message),
             .decodingFailed(let message):
            message
        }
    }

    var recoverySuggestion: String? {
        switch self {
        case .binaryNotFound:
            "Set the binary path in Settings > General, or install via Homebrew."
        case .setupNeeded:
            "Click Setup in the menu bar to initialize the database."
        case .notFound:
            "This item may have been removed. Try refreshing."
        case .commandFailed:
            "Check Settings > Diagnostics or the logs folder for details."
        default:
            nil
        }
    }
}

struct UserError: Equatable, Sendable {
    let message: String
    let recovery: String?

    init(_ error: Error) {
        self.message = error.localizedDescription
        self.recovery = (error as? ClipmemClientError)?.recoverySuggestion
    }

    init(message: String, recovery: String? = nil) {
        self.message = message
        self.recovery = recovery
    }
}

struct ClipmemClientConfiguration: Sendable {
    var binaryOverride: String?
    var databaseOverride: String?

    static var current: ClipmemClientConfiguration {
        let environment = ProcessInfo.processInfo.environment
        return ClipmemClientConfiguration(
            binaryOverride: UserDefaults.standard.string(forKey: PreferenceKey.binaryPathOverride),
            databaseOverride: environment["CLIPMEM_DB_PATH"]
                ?? UserDefaults.standard.string(forKey: PreferenceKey.databasePathOverride)
        )
    }
}

struct ClipmemClient: Sendable {
    var configuration: ClipmemClientConfiguration
    var runner: CommandRunner

    init(configuration: ClipmemClientConfiguration = .current, runner: CommandRunner = CommandRunner()) {
        self.configuration = configuration
        self.runner = runner
    }

    func serviceStatus() async throws -> ServiceStatusReport {
        try await decode(ServiceStatusReport.self, from: .serviceStatus(), timeout: .seconds(8))
    }

    func serviceRevision() async throws -> ArchiveRevision {
        try await decode(ArchiveRevision.self, from: .serviceRevision(), timeout: .seconds(4))
    }

    func doctor() async throws -> DoctorReport {
        try await decode(DoctorReport.self, from: .doctor())
    }

    func settings() async throws -> SettingsReport {
        try await decode(SettingsReport.self, from: .settingsShow())
    }

    func recent(limit: Int, cursor: String?, filters: RetrievalFilterState) async throws -> ListEnvelope {
        try await decode(ListEnvelope.self, from: .recent(limit: limit, cursor: cursor, filters: filters))
    }

    func timeline(limit: Int, cursor: String?, filters: RetrievalFilterState) async throws -> ListEnvelope {
        try await decode(ListEnvelope.self, from: .timeline(limit: limit, cursor: cursor, filters: filters))
    }

    func search(query: String, limit: Int, cursor: String?, filters: RetrievalFilterState) async throws -> ListEnvelope {
        try await decode(ListEnvelope.self, from: .search(query: query, limit: limit, cursor: cursor, filters: filters))
    }

    func recall(query: String?, limit: Int, filters: RetrievalFilterState) async throws -> RecallEnvelope {
        try await decode(RecallEnvelope.self, from: .recall(query: query, limit: limit, filters: filters))
    }

    func get(snapshotID: Int) async throws -> GetEnvelope {
        try await decode(GetEnvelope.self, from: .get(snapshotID: snapshotID))
    }

    func restore(snapshotID: Int) async throws -> RestoreOutput {
        try await decode(RestoreOutput.self, from: .restore(snapshotID: snapshotID))
    }

    func forget(snapshotID: Int) async throws -> ForgetOutput {
        try await decode(ForgetOutput.self, from: .forget(snapshotID: snapshotID))
    }

    func purge(olderThan: String, dryRun: Bool) async throws -> PurgeOutput {
        try await decode(PurgeOutput.self, from: .purge(olderThan: olderThan, dryRun: dryRun))
    }

    func storageCompact(dryRun: Bool) async throws -> StorageCompactOutput {
        try await decode(StorageCompactOutput.self, from: .storageCompact(dryRun: dryRun))
    }

    func storageOptimizeImages(dryRun: Bool, limit: Int?) async throws -> ImageOptimizationOutput {
        try await decode(ImageOptimizationOutput.self, from: .storageOptimizeImages(dryRun: dryRun, limit: limit))
    }

    func storageOptimizeImagesWithProgress(
        dryRun: Bool,
        limit: Int?,
        onProgress: @escaping @Sendable (ImageOptimizationProgressEvent) async -> Void
    ) async throws -> ImageOptimizationOutput {
        let reportBox = LockedBox<ImageOptimizationOutput>()
        try await runStreaming(.storageOptimizeImagesProgress(dryRun: dryRun, limit: limit)) { line in
            let data = Data(line.utf8)
            let event: ImageOptimizationProgressEvent
            do {
                event = try Self.decoder.decode(ImageOptimizationProgressEvent.self, from: data)
            } catch {
                throw ClipmemClientError.decodingFailed("Could not decode clipmem progress JSON for storage optimize-images.")
            }
            if case .complete(let report) = event {
                reportBox.set(report)
            }
            await onProgress(event)
        }
        guard let report = reportBox.value() else {
            throw ClipmemClientError.decodingFailed("clipmem storage optimize-images finished without a final progress report.")
        }
        return report
    }

    func export(snapshotID: Int, itemIndex: Int, uti: String, destination: String, force: Bool) async throws -> ExportOutput {
        try await decode(ExportOutput.self, from: .export(snapshotID: snapshotID, itemIndex: itemIndex, uti: uti, destination: destination, force: force))
    }

    func runAction(_ command: ClipmemCommand) async throws {
        _ = try await run(command)
    }

    func resolvedBinaryPath() -> String? {
        BinaryResolver(userOverride: configuration.binaryOverride).resolve()
    }

    func binaryCandidates() -> [String] {
        BinaryResolver(userOverride: configuration.binaryOverride).candidates()
    }

    private func decode<T: Decodable>(_ type: T.Type, from command: ClipmemCommand, timeout: Duration? = nil) async throws -> T {
        let result = try await run(command, timeout: timeout)
        do {
            return try Self.decoder.decode(T.self, from: result.stdout)
        } catch {
            throw ClipmemClientError.decodingFailed("Could not decode clipmem JSON for \(command.arguments.first ?? "command").")
        }
    }

    private func run(_ command: ClipmemCommand, timeout: Duration? = nil) async throws -> CommandResult {
        let resolver = BinaryResolver(userOverride: configuration.binaryOverride)
        guard let binary = resolver.resolve() else {
            throw ClipmemClientError.binaryNotFound(resolver.candidates())
        }
        let arguments = command.withDatabase(configuration.databaseOverride).arguments
        AppLoggers.commands.info("Running clipmem command: \(arguments.first ?? "unknown", privacy: .public)")
        let result = try await runner.run(executable: binary, arguments: arguments, timeout: timeout)
        if Task.isCancelled {
            throw CancellationError()
        }
        guard result.exitCode == 0 else {
            throw mapFailure(result)
        }
        return result
    }

    private func runStreaming(
        _ command: ClipmemCommand,
        onStdoutLine: @escaping @Sendable (String) async throws -> Void
    ) async throws {
        let resolver = BinaryResolver(userOverride: configuration.binaryOverride)
        guard let binary = resolver.resolve() else {
            throw ClipmemClientError.binaryNotFound(resolver.candidates())
        }
        let arguments = command.withDatabase(configuration.databaseOverride).arguments
        AppLoggers.commands.info("Running clipmem command: \(arguments.first ?? "unknown", privacy: .public)")
        let result = try await runner.runStreaming(
            executable: binary,
            arguments: arguments,
            onStdoutLine: onStdoutLine
        )
        if Task.isCancelled {
            throw CancellationError()
        }
        guard result.exitCode == 0 else {
            throw mapFailure(result)
        }
    }

    private func mapFailure(_ result: CommandResult) -> ClipmemClientError {
        let message = result.stderrText.trimmingCharacters(in: .whitespacesAndNewlines)
        let fallback = message.isEmpty ? "clipmem command failed with exit code \(result.exitCode)." : message
        switch result.exitCode {
        case 2:
            return .invalidArguments(fallback)
        case 3:
            return .notFound(fallback)
        case 4:
            return .unsupportedFormat(fallback)
        case 5:
            return .setupNeeded(fallback)
        case 6:
            return .platformError(fallback)
        default:
            return .commandFailed(result.exitCode, fallback)
        }
    }

    static let decoder: JSONDecoder = {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return decoder
    }()
}

private final class LockedBox<Value>: @unchecked Sendable {
    private let lock = NSLock()
    private var storedValue: Value?

    func set(_ value: Value) {
        lock.lock()
        storedValue = value
        lock.unlock()
    }

    func value() -> Value? {
        lock.lock()
        let value = storedValue
        lock.unlock()
        return value
    }
}
