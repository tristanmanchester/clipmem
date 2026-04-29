import Foundation

struct CommandResult: Sendable {
    var exitCode: Int32
    var stdout: Data
    var stderr: Data

    var stdoutText: String {
        String(data: stdout, encoding: .utf8) ?? ""
    }

    var stderrText: String {
        String(data: stderr, encoding: .utf8) ?? ""
    }
}

struct CommandRunner: Sendable {
    private let processStarted: (@Sendable () -> Void)?

    init(processStarted: (@Sendable () -> Void)? = nil) {
        self.processStarted = processStarted
    }

    func run(executable: String, arguments: [String]) async throws -> CommandResult {
        try await run(executable: executable, arguments: arguments, timeout: nil)
    }

    func run(executable: String, arguments: [String], timeout: Duration?) async throws -> CommandResult {
        let runningProcess = RunningProcess()
        let cancellationState = CancellationState()
        let processStarted = processStarted
        return try await withTaskCancellationHandler {
            let commandTask = Task.detached(priority: .userInitiated) {
                let process = Process()
                let stdout = Pipe()
                let stderr = Pipe()
                let stdoutReader = PipeReader(fileHandle: stdout.fileHandleForReading)
                let stderrReader = PipeReader(fileHandle: stderr.fileHandleForReading)

                process.executableURL = URL(fileURLWithPath: executable)
                process.arguments = arguments
                process.standardOutput = stdout
                process.standardError = stderr
                runningProcess.set(process)
                defer { runningProcess.clear() }

                stdoutReader.start()
                stderrReader.start()
                do {
                    try cancellationState.checkCancellation()
                } catch {
                    stdout.fileHandleForWriting.closeFile()
                    stderr.fileHandleForWriting.closeFile()
                    _ = stdoutReader.wait()
                    _ = stderrReader.wait()
                    throw error
                }

                do {
                    try process.run()
                } catch {
                    stdout.fileHandleForWriting.closeFile()
                    stderr.fileHandleForWriting.closeFile()
                    _ = stdoutReader.wait()
                    _ = stderrReader.wait()
                    throw error
                }

                processStarted?()
                process.waitUntilExit()
                let stdoutData = stdoutReader.wait()
                let stderrData = stderrReader.wait()
                try cancellationState.checkCancellation()
                return CommandResult(exitCode: process.terminationStatus, stdout: stdoutData, stderr: stderrData)
            }
            if let timeout {
                let timeoutTask = Task {
                    try? await Task.sleep(for: timeout)
                    if !Task.isCancelled {
                        cancellationState.cancel()
                        runningProcess.terminate()
                    }
                }
                defer { timeoutTask.cancel() }
                return try await commandTask.value
            }
            return try await commandTask.value
        } onCancel: {
            cancellationState.cancel()
            runningProcess.terminate()
        }
    }

    func runStreaming(
        executable: String,
        arguments: [String],
        onStdoutLine: @escaping @Sendable (String) async throws -> Void
    ) async throws -> CommandResult {
        let runningProcess = RunningProcess()
        let cancellationState = CancellationState()
        let pipeHandles = PipeHandles()
        let processStarted = processStarted
        return try await withTaskCancellationHandler {
            try await Task.detached(priority: .userInitiated) {
                let process = Process()
                let stdout = Pipe()
                let stderr = Pipe()
                let stderrReader = PipeReader(fileHandle: stderr.fileHandleForReading)

                process.executableURL = URL(fileURLWithPath: executable)
                process.arguments = arguments
                process.standardOutput = stdout
                process.standardError = stderr
                runningProcess.set(process)
                pipeHandles.set([
                    stdout.fileHandleForReading,
                    stdout.fileHandleForWriting,
                    stderr.fileHandleForReading,
                    stderr.fileHandleForWriting,
                ])
                defer { runningProcess.clear() }
                defer { pipeHandles.clear() }

                stderrReader.start()
                do {
                    try cancellationState.checkCancellation()
                    try process.run()
                    processStarted?()
                    let stdoutData = try await Self.consumeStdout(
                        from: stdout.fileHandleForReading,
                        cancellationState: cancellationState,
                        onStdoutLine: onStdoutLine
                    )
                    process.waitUntilExit()
                    let stderrData = stderrReader.wait()
                    try cancellationState.checkCancellation()
                    return CommandResult(exitCode: process.terminationStatus, stdout: stdoutData, stderr: stderrData)
                } catch {
                    process.terminate()
                    stdout.fileHandleForReading.closeFile()
                    stdout.fileHandleForWriting.closeFile()
                    stderr.fileHandleForReading.closeFile()
                    stderr.fileHandleForWriting.closeFile()
                    stderrReader.close()
                    if process.isRunning {
                        process.waitUntilExit()
                    }
                    throw error
                }
            }.value
        } onCancel: {
            cancellationState.cancel()
            runningProcess.terminate()
            pipeHandles.close()
        }
    }

    private static func consumeStdout(
        from fileHandle: FileHandle,
        cancellationState: CancellationState,
        onStdoutLine: @escaping @Sendable (String) async throws -> Void
    ) async throws -> Data {
        var output = Data()
        var pending = Data()

        while true {
            try cancellationState.checkCancellation()
            let chunk = fileHandle.availableData
            if chunk.isEmpty {
                break
            }
            output.append(chunk)
            pending.append(chunk)

            while let newline = pending.firstIndex(of: 0x0A) {
                let lineData = pending[..<newline]
                pending.removeSubrange(...newline)
                guard let line = String(data: lineData, encoding: .utf8) else {
                    throw ClipmemClientError.decodingFailed("Could not decode clipmem progress output.")
                }
                if !line.isEmpty {
                    try await onStdoutLine(line)
                }
            }
        }

        if !pending.isEmpty {
            guard let line = String(data: pending, encoding: .utf8) else {
                throw ClipmemClientError.decodingFailed("Could not decode clipmem progress output.")
            }
            try await onStdoutLine(line)
        }

        return output
    }
}

// Accessed by a cancellation handler and a worker task, so access is synchronized.
private final class RunningProcess: @unchecked Sendable {
    private let lock = NSLock()
    private var process: Process?

    func set(_ process: Process) {
        lock.lock()
        self.process = process
        lock.unlock()
    }

    func terminate() {
        lock.lock()
        let process = process
        lock.unlock()
        process?.terminate()
    }

    func clear() {
        lock.lock()
        process = nil
        lock.unlock()
    }
}

private final class CancellationState: @unchecked Sendable {
    private let lock = NSLock()
    private var cancelled = false

    func cancel() {
        lock.lock()
        cancelled = true
        lock.unlock()
    }

    func checkCancellation() throws {
        lock.lock()
        let cancelled = cancelled
        lock.unlock()
        if cancelled {
            throw CancellationError()
        }
    }
}

private final class PipeHandles: @unchecked Sendable {
    private let lock = NSLock()
    private var fileHandles: [FileHandle] = []

    func set(_ fileHandles: [FileHandle]) {
        lock.lock()
        self.fileHandles = fileHandles
        lock.unlock()
    }

    func close() {
        lock.lock()
        let fileHandles = fileHandles
        self.fileHandles = []
        lock.unlock()
        for fileHandle in fileHandles {
            fileHandle.closeFile()
        }
    }

    func clear() {
        lock.lock()
        fileHandles = []
        lock.unlock()
    }
}

private final class PipeReader: @unchecked Sendable {
    private let fileHandle: FileHandle
    private let semaphore = DispatchSemaphore(value: 0)
    private let lock = NSLock()
    private var data = Data()

    init(fileHandle: FileHandle) {
        self.fileHandle = fileHandle
    }

    func start() {
        DispatchQueue.global(qos: .userInitiated).async {
            let output = self.fileHandle.readDataToEndOfFile()
            self.lock.lock()
            self.data = output
            self.lock.unlock()
            self.semaphore.signal()
        }
    }

    func wait() -> Data {
        semaphore.wait()
        lock.lock()
        let output = data
        lock.unlock()
        return output
    }

    func close() {
        fileHandle.closeFile()
        semaphore.signal()
    }
}
