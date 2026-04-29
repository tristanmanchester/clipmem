import Foundation
import Testing
@testable import ClipmemMenuBar

struct DecodingTests {
    @Test func serviceStatusFixtureDecodesHealth() throws {
        let report = try decode(ServiceStatusReport.self, "service_status")

        #expect(report.health == .healthy)
        #expect(report.launchagent.running == true)
        #expect(report.retention == "30d")
        #expect(report.dbSizeBytes == 12_582_912)
        #expect(report.revision?.revision == 12)
        #expect(report.revision?.archiveContentRevision == 7)
        #expect(report.watcherBinaryPath == "/Users/test/clipmem/target/debug/clipmem")
    }

    @Test func stoppedWatcherFixtureMapsToStale() throws {
        let report = try decode(ServiceStatusReport.self, "service_status_stopped_watcher")

        #expect(report.stale == true)
        #expect(report.homebrew.running == false)
        #expect(report.launchagent.running == false)
        #expect(report.health == .stale)
    }

    @Test func serviceHealthMappingPrioritizesActionableStates() {
        let runningLaunchAgent = provider(.launchagent, installed: true, loaded: true, running: true)
        #expect(status(launchagent: runningLaunchAgent, recentCaptureWithinLastHour: true).health == .healthy)
        #expect(status(launchagent: runningLaunchAgent, recentCaptureWithinLastHour: false, stale: false).health == .noRecentCaptures)

        let stoppedLaunchAgent = provider(.launchagent, installed: true, loaded: true, running: false)
        #expect(status(launchagent: stoppedLaunchAgent, recentCaptureWithinLastHour: false, stale: true).health == .stale)
        #expect(status(launchagent: stoppedLaunchAgent, recentCaptureWithinLastHour: false, stale: false).health == .watcherStopped)

        let missingLaunchAgent = provider(.launchagent, installed: false, loaded: false, running: false)
        #expect(status(launchagent: missingLaunchAgent, recentCaptureWithinLastHour: false).health == .setupNeeded)

        #expect(status(conflict: true, launchagent: runningLaunchAgent, paused: true, stale: true).health == .conflict)
        #expect(status(launchagent: runningLaunchAgent, paused: true, stale: true, dbError: "database locked").health == .error)
        #expect(status(launchagent: runningLaunchAgent, dbExists: false, stale: true).health == .setupNeeded)
        #expect(status(launchagent: runningLaunchAgent, paused: true, stale: true).health == .capturePaused)
    }

    @Test func serviceStatusExposesWatcherBinaryMismatchWithoutChangingHealth() {
        let launchAgent = provider(
            .launchagent,
            installed: true,
            loaded: true,
            running: true,
            configuredBinaryPath: "/opt/homebrew/bin/clipmem",
            runningBinaryPath: "/opt/homebrew/bin/clipmem"
        )
        let report = status(
            launchagent: launchAgent,
            watcherBinaryMismatch: true,
            watcherBinaryMismatchNote: "launchagent watcher uses /opt/homebrew/bin/clipmem"
        )

        #expect(report.health == .healthy)
        #expect(report.watcherBinaryMismatch == true)
        #expect(report.watcherBinaryPath == "/opt/homebrew/bin/clipmem")
        #expect(report.watcherBinaryMismatchNote?.contains("/opt/homebrew/bin/clipmem") == true)
    }


    @Test func menuBarBadgePolicyMarksOnlyAttentionStates() {
        #expect(HealthState.healthy.menuBarBadgeSymbol == nil)
        #expect(HealthState.healthy.menuBarBadgeTone == nil)

        #expect(HealthState.stale.title == "Capture Stale")
        #expect(HealthState.stale.menuBarBadgeSymbol == "exclamationmark")
        #expect(HealthState.stale.menuBarBadgeTone == .warning)
        #expect(HealthState.setupNeeded.menuBarBadgeSymbol != nil)
        #expect(HealthState.setupNeeded.menuBarBadgeTone == .setup)
        #expect(HealthState.error.menuBarBadgeSymbol != nil)
        #expect(HealthState.error.menuBarBadgeTone == .critical)
        #expect(HealthState.conflict.menuBarBadgeSymbol != nil)
        #expect(HealthState.conflict.menuBarBadgeTone == .critical)
    }

    @Test func listEnvelopeFixtureDecodesRows() throws {
        let envelope = try decode(ListEnvelope.self, "recent")

        #expect(envelope.command == "recent")
        let firstResult = try #require(envelope.results.first)
        #expect(firstResult.displayText == "git status")
        #expect(firstResult.appHint == "Copied while in Terminal")
        #expect(firstResult.ocrText == "status from screenshot")
        #expect(firstResult.ocrStatus == "ready")
    }

    @Test func getFixtureDecodesNestedRepresentations() throws {
        let envelope = try decode(GetEnvelope.self, "get")

        #expect(envelope.snapshot.snapshotId == 7)
        let firstItem = try #require(envelope.snapshot.items.first)
        let firstRepresentation = try #require(firstItem.representations.first)
        let firstEvent = try #require(envelope.snapshot.recentEvents.first)
        #expect(envelope.snapshot.ocrText == "status from screenshot")
        #expect(envelope.snapshot.ocrStatus == "ready")
        #expect(firstRepresentation.uti == "public.utf8-plain-text")
        #expect(firstEvent.frontmostAppName == "Terminal")
    }

    @Test func settingsFixtureDecodesPolicy() throws {
        let settings = try decode(SettingsReport.self, "settings")

        #expect(settings.apiKeyFilterEnabled == true)
        #expect(settings.ocrEnabled == false)
        #expect(settings.ignoredBundleIds.contains("io.openclaw.clipmem.menubar"))
    }

    @Test func sqliteTimestampDisplaysInLocalTimeZone() throws {
        let berlin = try #require(TimeZone(identifier: "Europe/Berlin"))
        let formatted = try #require(DisplayFormatters.localTimestamp(
            "2026-04-19 06:20:00",
            timeZone: berlin,
            locale: Locale(identifier: "en_US_POSIX")
        ))

        #expect(formatted.contains("8:20"))
    }

    @Test func rfc3339TimestampDisplaysInLocalTimeZone() throws {
        let berlin = try #require(TimeZone(identifier: "Europe/Berlin"))
        let formatted = try #require(DisplayFormatters.localTimestamp(
            "2026-04-19T06:20:00Z",
            timeZone: berlin,
            locale: Locale(identifier: "en_US_POSIX")
        ))

        #expect(formatted.contains("8:20"))
    }

    @Test func actionPayloadsDecode() throws {
        let root = try decode([String: JSONValue].self, "actions")
        let data = try JSONSerialization.data(withJSONObject: try object(root["export"]))
        let export = try ClipmemClient.decoder.decode(ExportOutput.self, from: data)

        #expect(export.snapshotId == 7)
        #expect(export.uti == "public.png")
        #expect(export.byteCount == 42)

        let purgeData = try JSONSerialization.data(withJSONObject: try object(root["purge"]))
        let purge = try ClipmemClient.decoder.decode(PurgeOutput.self, from: purgeData)
        #expect(purge.dryRun == true)
        #expect(purge.olderThanSeconds == 2_592_000)
        #expect(purge.snapshotCount == 1)
        #expect(purge.itemCount == 1)
        #expect(purge.representationCount == 2)
        #expect(purge.captureEventCount == 3)
        #expect(purge.totalBytes == 42)

        let compactData = try JSONSerialization.data(withJSONObject: try object(root["storageCompact"]))
        let compact = try ClipmemClient.decoder.decode(StorageCompactOutput.self, from: compactData)
        #expect(compact.reclaimedBytes == 4096)
        #expect(compact.estimatedReclaimableBytes == 0)
        #expect(compact.checkpoint.busy == 0)

        let optimizeData = try JSONSerialization.data(withJSONObject: try object(root["imageOptimization"]))
        let optimize = try ClipmemClient.decoder.decode(ImageOptimizationOutput.self, from: optimizeData)
        #expect(optimize.format == "webp_lossless")
        #expect(optimize.compactRun == true)
        #expect(optimize.compact?.reclaimedBytes == 393_216)
        #expect(optimize.filesystemSavedBytes == 393_216)
        #expect(optimize.compactRecommended == false)
    }

    @Test func imageOptimizationProgressEventsDecode() throws {
        let events = try [
            #"{"type":"started","total_rows":3}"#,
            #"{"type":"scanning","scanned_rows":1,"total_rows":3,"compressed_rows":1,"skipped_rows":0,"conflict_count":0}"#,
            #"{"type":"compacting","scanned_rows":3,"total_rows":3,"compressed_rows":2,"skipped_rows":1,"conflict_count":0}"#,
            #"{"type":"complete","report":{"dry_run":false,"format":"webp_lossless","scanned_rows":3,"compressed_rows":2,"skipped_rows":1,"conflict_count":0,"original_bytes":100,"optimized_bytes":40,"logical_saved_bytes":60,"compact_run":true,"compact":null,"compact_error":null,"filesystem_saved_bytes":0,"filesystem_growth_bytes":0,"compact_recommended":false}}"#,
        ].map { line in
            try ClipmemClient.decoder.decode(ImageOptimizationProgressEvent.self, from: Data(line.utf8))
        }

        #expect(events[0] == .started(totalRows: 3))
        #expect(events[1] == .scanning(ImageOptimizationProgressSnapshot(
            scannedRows: 1,
            totalRows: 3,
            compressedRows: 1,
            skippedRows: 0,
            conflictCount: 0
        )))
        #expect(events[2] == .compacting(ImageOptimizationProgressSnapshot(
            scannedRows: 3,
            totalRows: 3,
            compressedRows: 2,
            skippedRows: 1,
            conflictCount: 0
        )))
        if case .complete(let report) = events[3] {
            #expect(report.scannedRows == 3)
            #expect(report.compressedRows == 2)
            #expect(report.compactRun == true)
        } else {
            Issue.record("Expected complete progress event.")
        }
    }

    private func decode<T: Decodable>(_ type: T.Type, _ name: String) throws -> T {
        let url = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("Fixtures")
            .appendingPathComponent("\(name).json")
        let data = try Data(contentsOf: url)
        return try ClipmemClient.decoder.decode(T.self, from: data)
    }

    private func status(
        conflict: Bool = false,
        homebrew: ProviderStatus? = nil,
        launchagent: ProviderStatus? = nil,
        dbExists: Bool = true,
        recentCaptureWithinLastHour: Bool? = true,
        paused: Bool? = false,
        stale: Bool = false,
        dbError: String? = nil,
        watcherBinaryMismatch: Bool = false,
        watcherBinaryMismatchNote: String? = nil
    ) -> ServiceStatusReport {
        ServiceStatusReport(
            binaryPath: "/Users/test/clipmem",
            dbPath: "/Users/test/clipmem.sqlite3",
            preferredProvider: .launchagent,
            preferredProviderReason: "test",
            conflict: conflict,
            homebrew: homebrew ?? provider(.homebrew, installed: false, loaded: false, running: false),
            launchagent: launchagent ?? provider(.launchagent, installed: true, loaded: true, running: true),
            dbExists: dbExists,
            dbSizeBytes: 1024,
            recentCaptureAt: "2026-04-20 08:09:29",
            recentCaptureWithinLastHour: recentCaptureWithinLastHour,
            paused: paused,
            apiKeyFilterEnabled: false,
            retentionSeconds: nil,
            retention: "forever",
            ignoredBundleIdCount: 0,
            revision: nil,
            stale: stale,
            dbError: dbError,
            watcherBinaryMismatch: watcherBinaryMismatch,
            watcherBinaryMismatchNote: watcherBinaryMismatchNote,
            notes: []
        )
    }

    private func provider(
        _ provider: ServiceProvider,
        installed: Bool,
        loaded: Bool,
        running: Bool,
        configuredBinaryPath: String? = nil,
        runningBinaryPath: String? = nil
    ) -> ProviderStatus {
        ProviderStatus(
            provider: provider,
            label: provider.rawValue,
            state: providerState(installed: installed, loaded: loaded, running: running),
            installed: installed,
            loaded: loaded,
            running: running,
            pid: running ? 123 : nil,
            plistPath: nil,
            configuredBinaryPath: configuredBinaryPath,
            runningCommand: runningBinaryPath.map { "\($0) watch --skip-initial" },
            runningBinaryPath: runningBinaryPath,
            stdoutLogPath: nil,
            stderrLogPath: nil
        )
    }

    private func providerState(installed: Bool, loaded: Bool, running: Bool) -> ServiceState {
        if running { return .running }
        if loaded { return .loaded }
        if installed { return .installed }
        return .notInstalled
    }

    private func object(_ value: JSONValue?) throws -> [String: Any] {
        guard case .object(let dictionary) = value else {
            throw FixtureError.expectedObject
        }
        return dictionary.mapValues(any)
    }

    private func any(_ value: JSONValue) -> Any {
        switch value {
        case .string(let value): value
        case .int(let value): value
        case .double(let value): value
        case .bool(let value): value
        case .object(let value): value.mapValues(any)
        case .array(let value): value.map(any)
        case .null: NSNull()
        }
    }

    enum FixtureError: Error {
        case expectedObject
    }
}
