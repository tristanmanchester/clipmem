import Foundation

struct ClipmemCommand: Equatable, Sendable {
    var arguments: [String]

    func withDatabase(_ databasePath: String?) -> ClipmemCommand {
        guard let databasePath, databasePath.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false else {
            return self
        }
        return ClipmemCommand(arguments: ["--db", databasePath] + arguments)
    }

    static func serviceStatus() -> ClipmemCommand {
        ClipmemCommand(arguments: ["service", "status", "--json"])
    }

    static func serviceRevision() -> ClipmemCommand {
        ClipmemCommand(arguments: ["service", "revision", "--format", "json"])
    }

    static func doctor() -> ClipmemCommand {
        ClipmemCommand(arguments: ["doctor", "--json"])
    }

    static func setup() -> ClipmemCommand {
        ClipmemCommand(arguments: ["setup"])
    }

    static func service(_ action: String) -> ClipmemCommand {
        ClipmemCommand(arguments: ["service", action])
    }

    static func settingsShow() -> ClipmemCommand {
        ClipmemCommand(arguments: ["settings", "show", "--format", "json"])
    }

    static func settingsIgnoreList() -> ClipmemCommand {
        ClipmemCommand(arguments: ["settings", "ignore", "list", "--format", "json"])
    }

    static func settingsIgnoreAdd(_ bundleID: String) -> ClipmemCommand {
        ClipmemCommand(arguments: ["settings", "ignore", "add", bundleID])
    }

    static func settingsIgnoreRemove(_ bundleID: String) -> ClipmemCommand {
        ClipmemCommand(arguments: ["settings", "ignore", "remove", bundleID])
    }

    static func settingsPause(_ paused: Bool) -> ClipmemCommand {
        ClipmemCommand(arguments: ["settings", "pause", paused ? "on" : "off"])
    }

    static func settingsAPIKeyFilter(_ enabled: Bool) -> ClipmemCommand {
        ClipmemCommand(arguments: ["settings", "api-key-filter", enabled ? "on" : "off"])
    }

    static func settingsOCR(_ enabled: Bool) -> ClipmemCommand {
        ClipmemCommand(arguments: ["settings", "ocr", enabled ? "on" : "off"])
    }

    static func settingsRetention(_ value: String) -> ClipmemCommand {
        ClipmemCommand(arguments: ["settings", "retention", value])
    }

    static func appSettingsShow() -> ClipmemCommand {
        ClipmemCommand(arguments: ["app", "settings", "show", "--format", "json"])
    }

    static func appSettingsSet(_ key: String, value: String) -> ClipmemCommand {
        ClipmemCommand(arguments: ["app", "settings", "set", key, value, "--format", "json"])
    }

    static func appSettingsClear(_ key: String) -> ClipmemCommand {
        ClipmemCommand(arguments: ["app", "settings", "clear", key, "--format", "json"])
    }

    static func appLaunchAtLoginShow() -> ClipmemCommand {
        ClipmemCommand(arguments: ["app", "launch-at-login", "show", "--format", "json"])
    }

    static func appLaunchAtLoginSet(_ enabled: Bool) -> ClipmemCommand {
        ClipmemCommand(arguments: ["app", "launch-at-login", "set", enabled ? "on" : "off", "--format", "json"])
    }

    static func appLaunchAtLoginClear() -> ClipmemCommand {
        ClipmemCommand(arguments: ["app", "launch-at-login", "clear", "--format", "json"])
    }

    static func appUpdateCheckShow() -> ClipmemCommand {
        ClipmemCommand(arguments: ["app", "update-check", "show", "--format", "json"])
    }

    static func appUpdateCheckRun() -> ClipmemCommand {
        ClipmemCommand(arguments: ["app", "update-check", "run", "--format", "json"])
    }

    static func appUpdateCheckClear() -> ClipmemCommand {
        ClipmemCommand(arguments: ["app", "update-check", "clear", "--format", "json"])
    }

    static func appQuit() -> ClipmemCommand {
        ClipmemCommand(arguments: ["app", "quit", "--format", "json"])
    }

    static func recent(limit: Int, cursor: String?, filters: RetrievalFilterState) -> ClipmemCommand {
        listCommand(["recent"], limit: limit, cursor: cursor, filters: filters)
    }

    static func timeline(limit: Int, cursor: String?, filters: RetrievalFilterState) -> ClipmemCommand {
        listCommand(["timeline"], limit: limit, cursor: cursor, filters: filters)
    }

    static func search(query: String, limit: Int, cursor: String?, filters: RetrievalFilterState) -> ClipmemCommand {
        var arguments = ["search", "--limit", String(limit), "--format", "json"]
        if let cursor, cursor.isEmpty == false {
            arguments += ["--cursor", cursor]
        }
        appendFilters(&arguments, filters: filters)
        arguments += ["--", query]
        return ClipmemCommand(arguments: arguments)
    }

    static func recall(query: String?, limit: Int, filters: RetrievalFilterState) -> ClipmemCommand {
        var arguments = ["recall", "--limit", String(limit), "--format", "json"]
        appendFilters(&arguments, filters: filters)
        if let query, query.isEmpty == false {
            arguments += ["--", query]
        } else {
            arguments.append("--prefer-recent")
        }
        return ClipmemCommand(arguments: arguments)
    }

    static func get(snapshotID: Int) -> ClipmemCommand {
        ClipmemCommand(arguments: ["get", String(snapshotID), "--events", "25", "--format", "json"])
    }

    static func restore(snapshotID: Int) -> ClipmemCommand {
        ClipmemCommand(arguments: ["restore", String(snapshotID), "--format", "json"])
    }

    static func forget(snapshotID: Int) -> ClipmemCommand {
        ClipmemCommand(arguments: ["forget", String(snapshotID), "--format", "json"])
    }

    static func purge(olderThan: String, dryRun: Bool) -> ClipmemCommand {
        var arguments = ["purge", "--older-than", olderThan, "--format", "json"]
        if dryRun {
            arguments.append("--dry-run")
        }
        return ClipmemCommand(arguments: arguments)
    }

    static func storageCompact(dryRun: Bool) -> ClipmemCommand {
        var arguments = ["storage", "compact", "--format", "json"]
        if dryRun {
            arguments.append("--dry-run")
        }
        return ClipmemCommand(arguments: arguments)
    }

    static func storageOptimizeImages(dryRun: Bool, limit: Int?) -> ClipmemCommand {
        var arguments = ["storage", "optimize-images", "--format", "json"]
        if dryRun {
            arguments.append("--dry-run")
        }
        if let limit {
            arguments += ["--limit", String(limit)]
        }
        return ClipmemCommand(arguments: arguments)
    }

    static func storageOptimizeImagesProgress(dryRun: Bool, limit: Int?) -> ClipmemCommand {
        var arguments = ["storage", "optimize-images", "--progress", "jsonl"]
        if dryRun {
            arguments.append("--dry-run")
        }
        if let limit {
            arguments += ["--limit", String(limit)]
        }
        return ClipmemCommand(arguments: arguments)
    }

    static func export(snapshotID: Int, itemIndex: Int, uti: String, destination: String, force: Bool) -> ClipmemCommand {
        var arguments = [
            "export",
            String(snapshotID),
            "--item",
            String(itemIndex),
            "--uti",
            uti,
            "--out",
            destination,
            "--format",
            "json"
        ]
        if force {
            arguments.append("--force")
        }
        return ClipmemCommand(arguments: arguments)
    }

    private static func listCommand(
        _ prefix: [String],
        limit: Int,
        cursor: String?,
        filters: RetrievalFilterState
    ) -> ClipmemCommand {
        var arguments = prefix + ["--limit", String(limit), "--format", "json"]
        if let cursor, cursor.isEmpty == false {
            arguments += ["--cursor", cursor]
        }
        appendFilters(&arguments, filters: filters)
        return ClipmemCommand(arguments: arguments)
    }

    private static func appendFilters(_ arguments: inout [String], filters: RetrievalFilterState) {
        if filters.hours > 0 {
            arguments += ["--hours", String(filters.hours)]
        }
        if filters.appName.isEmpty == false {
            arguments += ["--app", filters.appName]
        }
        if filters.bundleID.isEmpty == false {
            arguments += ["--bundle-id", filters.bundleID]
        }
        if let kind = filters.kind {
            arguments += ["--kind", kind.rawValue]
        }
        if filters.hasText { arguments.append("--has-text") }
        if filters.hasURL { arguments.append("--has-url") }
        if filters.hasFile { arguments.append("--has-file-url") }
        if filters.hasImage { arguments.append("--has-image") }
        if filters.hasPDF { arguments.append("--has-pdf") }
    }
}
