import Foundation
import Testing
@testable import ClipmemMenuBar

@Suite(.serialized)
struct CommandRunnerTests {
    @Test func drainsLargeStdoutAndStderrBeforeWaiting() async throws {
        let byteCount = 200_000
        let script = "print \"o\" x \(byteCount); print STDERR \"e\" x \(byteCount);"

        let result = try await CommandRunner().run(executable: "/usr/bin/perl", arguments: ["-e", script])

        #expect(result.exitCode == 0)
        #expect(result.stdout.count == byteCount)
        #expect(result.stderr.count == byteCount)
    }

    @Test func cancellationTerminatesRunningProcess() async throws {
        let processStarted = AsyncSignal()
        let runner = CommandRunner(processStarted: {
            processStarted.signal()
        })
        let task = Task {
            try await runner.run(executable: "/bin/sh", arguments: ["-c", "exec sleep 30"])
        }

        await processStarted.wait()
        task.cancel()

        do {
            _ = try await task.value
            Issue.record("Expected cancellation to throw.")
        } catch is CancellationError {
        } catch {
            Issue.record("Expected CancellationError, got \(error).")
        }
    }

    @Test func streamingRunEmitsStdoutLinesBeforeCompletion() async throws {
        let script = "printf 'one\\n'; sleep 0.1; printf 'two\\n'"
        let lines = LockedStringList()

        let result = try await CommandRunner().runStreaming(
            executable: "/bin/sh",
            arguments: ["-c", script]
        ) { line in
            lines.append(line)
        }

        #expect(result.exitCode == 0)
        #expect(lines.values() == ["one", "two"])
        #expect(String(data: result.stdout, encoding: .utf8) == "one\ntwo\n")
    }

    @Test func streamingRunPropagatesLineHandlerErrors() async throws {
        let task = Task {
            try await CommandRunner().runStreaming(
                executable: "/bin/sh",
                arguments: ["-c", "printf 'bad\\n'; exec sleep 30"]
            ) { _ in
                throw ClipmemClientError.decodingFailed("bad line")
            }
        }

        do {
            _ = try await task.value
            Issue.record("Expected line handler failure.")
        } catch let error as ClipmemClientError {
            #expect(error == .decodingFailed("bad line"))
        } catch {
            Issue.record("Expected decoding failure, got \(error).")
        }
    }

    @Test func streamingRunCancellationTerminatesRunningProcess() async throws {
        let processStarted = AsyncSignal()
        let runner = CommandRunner(processStarted: {
            processStarted.signal()
        })
        let task = Task {
            try await runner.runStreaming(
                executable: "/bin/sh",
                arguments: ["-c", "exec sleep 30"]
            ) { _ in }
        }

        await processStarted.wait()
        task.cancel()

        do {
            _ = try await task.value
            Issue.record("Expected cancellation to throw.")
        } catch is CancellationError {
        } catch {
            Issue.record("Expected CancellationError, got \(error).")
        }
    }

    private final class AsyncSignal: @unchecked Sendable {
        private let lock = NSLock()
        private var isSignaled = false
        private var continuations: [CheckedContinuation<Void, Never>] = []

        func wait() async {
            await withCheckedContinuation { continuation in
                lock.lock()
                if isSignaled {
                    lock.unlock()
                    continuation.resume()
                } else {
                    continuations.append(continuation)
                    lock.unlock()
                }
            }
        }

        func signal() {
            lock.lock()
            if isSignaled {
                lock.unlock()
                return
            }
            isSignaled = true
            let continuations = continuations
            self.continuations.removeAll()
            lock.unlock()

            for continuation in continuations {
                continuation.resume()
            }
        }
    }

    private final class LockedStringList: @unchecked Sendable {
        private let lock = NSLock()
        private var strings: [String] = []

        func append(_ string: String) {
            lock.lock()
            strings.append(string)
            lock.unlock()
        }

        func values() -> [String] {
            lock.lock()
            let values = strings
            lock.unlock()
            return values
        }
    }
}

@MainActor
struct ReactiveRefreshTests {
    @Test func pasteboardMonitorEmitsOnlyWhenChangeCountChanges() {
        let changeCount = IntBox(1)
        let emittedChanges = IntBox(0)
        let monitor = PasteboardChangeMonitor(
            changeCount: { changeCount.value },
            onChange: { emittedChanges.value += 1 }
        )

        monitor.pollOnce()
        monitor.pollOnce()
        changeCount.value = 2
        monitor.pollOnce()
        monitor.pollOnce()

        #expect(emittedChanges.value == 1)
    }

    @Test func pasteboardMonitorCanMarkCurrentChangeHandled() {
        let changeCount = IntBox(1)
        let emittedChanges = IntBox(0)
        let monitor = PasteboardChangeMonitor(
            changeCount: { changeCount.value },
            onChange: { emittedChanges.value += 1 }
        )

        monitor.pollOnce()
        changeCount.value = 2
        monitor.markCurrentChangeHandled()
        monitor.pollOnce()

        #expect(emittedChanges.value == 0)
    }

    @Test func recentRefreshCoordinatorCoalescesRapidChanges() async {
        var refreshCount = 0
        let coordinator = RecentPreviewRefreshCoordinator(
            sleep: { _ in },
            refresh: {
                refreshCount += 1
                return true
            }
        )

        coordinator.schedule()
        coordinator.schedule()
        coordinator.schedule()
        await Self.drainScheduledTasks()

        #expect(refreshCount == 1)
    }

    @Test func recentRefreshCoordinatorQueuesOneFollowUpWhileRefreshing() async {
        var refreshCount = 0
        var firstRefreshContinuation: CheckedContinuation<Void, Never>?
        let coordinator = RecentPreviewRefreshCoordinator(
            sleep: { _ in },
            refresh: {
                refreshCount += 1
                if refreshCount == 1 {
                    await withCheckedContinuation { continuation in
                        firstRefreshContinuation = continuation
                    }
                }
                return true
            }
        )

        coordinator.schedule()
        await Self.drainScheduledTasks()

        coordinator.schedule()
        coordinator.schedule()
        await Self.drainScheduledTasks()
        #expect(refreshCount == 1)

        firstRefreshContinuation?.resume()
        await Self.drainScheduledTasks()

        #expect(refreshCount == 2)
    }

    @Test func staleRecentPreviewRefreshIncrementsRevisionOnlyWhenItRefreshes() async {
        var loadCount = 0
        let appModel = AppModel {
            loadCount += 1
            return [Self.item(9)]
        }

        await appModel.refreshRecentPreviewIfStale(maxAge: 1)
        await appModel.refreshRecentPreviewIfStale(maxAge: 60)

        #expect(loadCount == 1)
        #expect(appModel.clipboardHistoryRevision == 1)
        #expect(appModel.recentPreview.map(\.snapshotId) == [9])
    }

    @Test func recentPreviewRefreshReportsOnlyActualListChanges() async {
        var loads = [[Self.item(9)], [Self.item(9)], [Self.item(10)]]
        let appModel = AppModel {
            loads.removeFirst()
        }

        let firstChanged = await appModel.refreshRecentPreview()
        let secondChanged = await appModel.refreshRecentPreview()
        let thirdChanged = await appModel.refreshRecentPreview()

        #expect(firstChanged)
        #expect(!secondChanged)
        #expect(thirdChanged)
        #expect(appModel.recentPreview.map(\.snapshotId) == [10])
    }

    private static func drainScheduledTasks() async {
        for _ in 0..<5 {
            await Task.yield()
        }
    }

    private static func item(_ snapshotID: Int) -> ClipmemItem {
        ClipmemItem(
            snapshotId: snapshotID,
            eventId: nil,
            sha256: nil,
            kind: .plainText,
            observedAt: nil,
            firstSeenAt: nil,
            lastSeenAt: nil,
            appName: nil,
            appBundleId: nil,
            bestText: nil,
            bestTextUti: nil,
            textFragments: nil,
            urls: nil,
            filePaths: nil,
            htmlText: nil,
            rtfText: nil,
            textSummary: nil,
            ocrText: nil,
            ocrStatus: nil,
            previewText: nil,
            itemCount: nil,
            totalBytes: nil,
            captureCount: nil,
            score: nil,
            whyMatched: nil,
            matchedFields: nil,
            snippet: nil,
            changeCount: nil
        )
    }

    private final class IntBox {
        var value: Int

        init(_ value: Int) {
            self.value = value
        }
    }
}

@MainActor
struct HistoryExternalRefreshTests {
    @Test(arguments: [QueryMode.recent, .timeline])
    func externalHistoryRefreshReloadsBrowseModesAndPreservesSelection(mode: QueryMode) async {
        var requestedModes: [QueryMode] = []
        let history = HistoryModel(mode: mode, appModel: AppModel()) { mode, _, _, _ in
            requestedModes.append(mode)
            return ([Self.item(3), Self.item(2), Self.item(1)], "next")
        }
        history.results = [Self.item(2), Self.item(1)]
        history.selectedID = 2

        await history.refreshForExternalHistoryChange()

        #expect(requestedModes == [mode])
        #expect(history.results.map(\.snapshotId) == [3, 2, 1])
        #expect(history.nextCursor == "next")
        #expect(history.selectedID == 2)
    }

    @Test func externalHistoryRefreshSelectsNewestWhenPreviousSelectionDisappears() async {
        let history = HistoryModel(mode: .recent, appModel: AppModel()) { _, _, _, _ in
            ([Self.item(4), Self.item(3)], nil)
        }
        history.results = [Self.item(2), Self.item(1)]
        history.selectedID = 2

        await history.refreshForExternalHistoryChange()

        #expect(history.results.map(\.snapshotId) == [4, 3])
        #expect(history.selectedID == 4)
        #expect(history.selectedDetail == nil)
    }

    @Test(arguments: [QueryMode.recall, .search])
    func externalHistoryRefreshReloadsQueryModesWithoutClearingQuery(mode: QueryMode) async {
        var requestedModes: [QueryMode] = []
        var requestedQueries: [String] = []
        var loadCount = 0
        let history = HistoryModel(mode: mode, appModel: AppModel()) { mode, query, _, _ in
            requestedModes.append(mode)
            requestedQueries.append(query)
            loadCount += 1
            return ([Self.item(3)], nil)
        }
        history.query = "needle"
        history.results = [Self.item(1)]
        history.selectedID = 1

        await history.refreshForExternalHistoryChange()

        #expect(loadCount == 1)
        #expect(requestedModes == [mode])
        #expect(requestedQueries == ["needle"])
        #expect(history.query == "needle")
        #expect(history.results.map(\.snapshotId) == [3])
        #expect(history.selectedID == 3)
    }

    @Test func externalHistoryRefreshIgnoresDiagnosticsMode() async {
        var loadCount = 0
        let history = HistoryModel(mode: .diagnostics, appModel: AppModel()) { _, _, _, _ in
            loadCount += 1
            return ([Self.item(3)], nil)
        }
        history.results = [Self.item(1)]
        history.selectedID = 1

        await history.refreshForExternalHistoryChange()

        #expect(loadCount == 0)
        #expect(history.results.map(\.snapshotId) == [1])
        #expect(history.selectedID == 1)
    }

    private static func item(_ snapshotID: Int) -> ClipmemItem {
        ClipmemItem(
            snapshotId: snapshotID,
            eventId: nil,
            sha256: nil,
            kind: .plainText,
            observedAt: nil,
            firstSeenAt: nil,
            lastSeenAt: nil,
            appName: nil,
            appBundleId: nil,
            bestText: nil,
            bestTextUti: nil,
            textFragments: nil,
            urls: nil,
            filePaths: nil,
            htmlText: nil,
            rtfText: nil,
            textSummary: nil,
            ocrText: nil,
            ocrStatus: nil,
            previewText: nil,
            itemCount: nil,
            totalBytes: nil,
            captureCount: nil,
            score: nil,
            whyMatched: nil,
            matchedFields: nil,
            snippet: nil,
            changeCount: nil
        )
    }
}

@MainActor
struct QuickRecallModelTests {
    @Test func forgetExplicitItemDoesNotDependOnSelection() async {
        var forgottenIDs: [Int] = []
        let model = QuickRecallModel(appModel: AppModel()) { item in
            forgottenIDs.append(item.snapshotId)
            return true
        }
        model.results = [Self.item(1), Self.item(2)]
        model.selectedID = 1

        await model.forget(Self.item(2))

        #expect(forgottenIDs == [2])
        #expect(model.results.map(\.snapshotId) == [1])
        #expect(model.selectedID == 1)
    }

    @Test func failedForgetLeavesResultsAndSelectionUnchanged() async {
        var forgottenIDs: [Int] = []
        let model = QuickRecallModel(appModel: AppModel()) { item in
            forgottenIDs.append(item.snapshotId)
            return false
        }
        model.results = [Self.item(1), Self.item(2)]
        model.selectedID = 2

        await model.forget(Self.item(2))

        #expect(forgottenIDs == [2])
        #expect(model.results.map(\.snapshotId) == [1, 2])
        #expect(model.selectedID == 2)
    }

    @Test func copyablePlainTextUsesFirstNonEmptyTextValue() {
        #expect(Self.item(1, bestText: "plain", previewText: "preview").copyablePlainText == "plain")
        #expect(Self.item(1, bestText: nil, previewText: "preview").copyablePlainText == "preview")
        #expect(Self.item(1, bestText: "", previewText: "preview").copyablePlainText == "preview")
        #expect(Self.item(1, bestText: nil, previewText: nil).copyablePlainText == nil)
        #expect(Self.item(1, bestText: "", previewText: "").copyablePlainText == nil)
    }

    private static func item(_ snapshotID: Int, bestText: String? = nil, previewText: String? = nil) -> ClipmemItem {
        ClipmemItem(
            snapshotId: snapshotID,
            eventId: nil,
            sha256: nil,
            kind: .plainText,
            observedAt: nil,
            firstSeenAt: nil,
            lastSeenAt: nil,
            appName: nil,
            appBundleId: nil,
            bestText: bestText,
            bestTextUti: nil,
            textFragments: nil,
            urls: nil,
            filePaths: nil,
            htmlText: nil,
            rtfText: nil,
            textSummary: nil,
            ocrText: nil,
            ocrStatus: nil,
            previewText: previewText,
            itemCount: nil,
            totalBytes: nil,
            captureCount: nil,
            score: nil,
            whyMatched: nil,
            matchedFields: nil,
            snippet: nil,
            changeCount: nil
        )
    }
}
