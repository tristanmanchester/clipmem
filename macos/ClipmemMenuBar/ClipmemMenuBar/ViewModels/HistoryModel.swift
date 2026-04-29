import Foundation
import Observation

typealias HistoryPage = (items: [ClipmemItem], nextCursor: String?)
typealias HistoryPageLoader = @MainActor (QueryMode, String, RetrievalFilterState, String?) async throws -> HistoryPage
typealias HistoryDetailLoader = @MainActor (Int) async throws -> SnapshotDetails

@MainActor
@Observable
final class HistoryModel {
    var mode: QueryMode
    var query = ""
    var filters = RetrievalFilterState.defaultValue
    var results: [ClipmemItem] = []
    var selectedID: Int?
    var selectedDetail: SnapshotDetails?
    var nextCursor: String?
    var isLoading = false
    var isLoadingDetail = false
    var error: UserError?

    @ObservationIgnored private let appModel: AppModel
    @ObservationIgnored private let pageLoader: HistoryPageLoader?
    @ObservationIgnored private let detailLoader: HistoryDetailLoader
    @ObservationIgnored private var loadGeneration = 0
    @ObservationIgnored private var detailGeneration = 0

    init(
        mode: QueryMode = UserDefaults.standard.clipmemDefaultMode,
        appModel: AppModel,
        pageLoader: HistoryPageLoader? = nil,
        detailLoader: HistoryDetailLoader? = nil
    ) {
        self.mode = mode
        self.appModel = appModel
        self.pageLoader = pageLoader
        self.detailLoader = detailLoader ?? { snapshotID in
            try await appModel.client.get(snapshotID: snapshotID).snapshot
        }
    }

    var selectedItem: ClipmemItem? {
        guard let selectedID else { return nil }
        return results.first { $0.snapshotId == selectedID }
    }

    func reload(selecting snapshotID: Int? = nil) async {
        loadGeneration += 1
        let generation = loadGeneration
        nextCursor = nil
        results = []
        selectedID = snapshotID
        selectedDetail = nil
        await loadMore(generation: generation)
    }

    func loadMore() async {
        loadGeneration += 1
        await loadMore(generation: loadGeneration)
    }

    func refreshForExternalHistoryChange() async {
        guard mode != .diagnostics else { return }

        loadGeneration += 1
        let generation = loadGeneration
        let previousSelectedID = selectedID
        let request = HistoryRequest(
            generation: generation,
            mode: mode,
            query: query,
            filters: filters,
            cursor: nil
        )

        isLoading = true
        defer {
            if generation == loadGeneration {
                isLoading = false
            }
        }

        do {
            let page = try await loadPage(request)
            guard isCurrent(request) else { return }
            results = page.items
            nextCursor = page.nextCursor
            if let previousSelectedID, results.contains(where: { $0.snapshotId == previousSelectedID }) {
                selectedID = previousSelectedID
            } else {
                selectedID = results.first?.snapshotId
                selectedDetail = nil
            }
            error = nil
        } catch is CancellationError {
        } catch {
            guard isCurrent(request) else { return }
            self.error = UserError(error)
        }
    }

    private func loadMore(generation: Int) async {
        guard mode != .diagnostics else {
            await appModel.refreshDoctor()
            return
        }
        let request = HistoryRequest(
            generation: generation,
            mode: mode,
            query: query,
            filters: filters,
            cursor: nextCursor
        )
        isLoading = true
        defer {
            if generation == loadGeneration {
                isLoading = false
            }
        }
        do {
            let page = try await loadPage(request)
            guard isCurrent(request) else { return }
            if request.cursor == nil {
                results = page.items
            } else {
                results.append(contentsOf: page.items)
            }
            nextCursor = page.nextCursor
            if selectedID == nil {
                selectedID = results.first?.snapshotId
            }
            if selectedID != nil, selectedDetail == nil {
                await loadSelectedDetail()
            }
            error = nil
        } catch is CancellationError {
        } catch {
            guard isCurrent(request) else { return }
            self.error = UserError(error)
        }
    }

    func loadSelectedDetail() async {
        detailGeneration += 1
        let generation = detailGeneration
        guard let selectedID else {
            selectedDetail = nil
            return
        }
        isLoadingDetail = true
        defer {
            if generation == detailGeneration {
                isLoadingDetail = false
            }
        }
        do {
            let detail = try await detailLoader(selectedID)
            guard generation == detailGeneration, self.selectedID == selectedID else { return }
            selectedDetail = detail
            error = nil
        } catch is CancellationError {
        } catch {
            guard generation == detailGeneration, self.selectedID == selectedID else { return }
            selectedDetail = nil
            self.error = UserError(error)
        }
    }

    func restoreSelected() async {
        guard let selectedItem else { return }
        await appModel.restore(selectedItem)
    }

    func forgetSelected() async {
        guard let selectedItem else { return }
        guard await appModel.forget(selectedItem) else { return }
        results.removeAll { $0.snapshotId == selectedItem.snapshotId }
        selectedID = results.first?.snapshotId
        await loadSelectedDetail()
    }

    private func loadPage(_ request: HistoryRequest) async throws -> HistoryPage {
        if let pageLoader {
            return try await pageLoader(request.mode, request.query, request.filters, request.cursor)
        }
        switch request.mode {
        case .recall:
            let envelope = try await appModel.client.recall(query: request.query.isEmpty ? nil : request.query, limit: 25, filters: request.filters)
            return ([envelope.bestCandidate] + envelope.alternatives, nil)
        case .search:
            let envelope = try await appModel.client.search(query: request.query, limit: 40, cursor: request.cursor, filters: request.filters)
            return (envelope.results, envelope.nextCursor)
        case .recent:
            let envelope = try await appModel.client.recent(limit: 40, cursor: request.cursor, filters: request.filters)
            return (envelope.results, envelope.nextCursor)
        case .timeline:
            let envelope = try await appModel.client.timeline(limit: 40, cursor: request.cursor, filters: request.filters)
            return (envelope.results, envelope.nextCursor)
        case .diagnostics:
            return ([], nil)
        }
    }

    private func isCurrent(_ request: HistoryRequest) -> Bool {
        request.generation == loadGeneration
            && request.mode == mode
            && request.query == query
            && request.filters == filters
    }
}

private struct HistoryRequest: Equatable, Sendable {
    var generation: Int
    var mode: QueryMode
    var query: String
    var filters: RetrievalFilterState
    var cursor: String?
}
