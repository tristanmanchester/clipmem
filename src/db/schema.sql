PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS snapshots (
    id            INTEGER PRIMARY KEY,
    sha256        TEXT NOT NULL UNIQUE,
    snapshot_kind TEXT NOT NULL,
    preview_text  TEXT NOT NULL,
    search_text   TEXT NOT NULL,
    item_count    INTEGER NOT NULL CHECK (item_count >= 0),
    total_bytes   INTEGER NOT NULL CHECK (total_bytes >= 0),
    created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS snapshot_items (
    snapshot_id   INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    item_index    INTEGER NOT NULL CHECK (item_index >= 0),
    primary_kind  TEXT NOT NULL,
    primary_uti   TEXT,
    preview_text  TEXT NOT NULL,
    search_text   TEXT NOT NULL,
    total_bytes   INTEGER NOT NULL CHECK (total_bytes >= 0),
    PRIMARY KEY (snapshot_id, item_index)
);

CREATE TABLE IF NOT EXISTS item_representations (
    snapshot_id    INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    item_index     INTEGER NOT NULL CHECK (item_index >= 0),
    uti            TEXT NOT NULL,
    kind           TEXT NOT NULL,
    byte_len       INTEGER NOT NULL CHECK (byte_len >= 0),
    raw_sha256     TEXT NOT NULL,
    text_value     TEXT,
    blob_value     BLOB NOT NULL,
    image_compression_status TEXT NOT NULL DEFAULT 'uncompressed' CHECK (image_compression_status IN ('uncompressed', 'compressed', 'skipped')),
    image_compression_format TEXT,
    image_compressed_at TEXT,
    image_original_byte_len INTEGER,
    image_original_raw_sha256 TEXT,
    image_compression_reason TEXT,
    PRIMARY KEY (snapshot_id, item_index, uti)
);

CREATE TABLE IF NOT EXISTS capture_events (
    id                     INTEGER PRIMARY KEY,
    snapshot_id            INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    observed_at            TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    change_count           INTEGER NOT NULL CHECK (change_count >= 0),
    frontmost_app_bundle_id TEXT,
    frontmost_app_name     TEXT
);

CREATE TABLE IF NOT EXISTS snapshot_stats (
    snapshot_id                 INTEGER PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE,
    capture_count               INTEGER NOT NULL CHECK (capture_count >= 0),
    first_observed_at           TEXT NOT NULL,
    last_observed_at            TEXT NOT NULL,
    last_event_id               INTEGER NOT NULL,
    last_frontmost_app_bundle_id TEXT,
    last_frontmost_app_name     TEXT
);

CREATE TABLE IF NOT EXISTS snapshot_projection_cache (
    snapshot_id INTEGER PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE,
    urls        TEXT NOT NULL DEFAULT '',
    file_urls   TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS snapshot_event_filter_cache (
    snapshot_id       INTEGER PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE,
    app_names_lower   TEXT NOT NULL DEFAULT '',
    bundle_ids_lower  TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS snapshot_literal_cache (
    snapshot_id INTEGER PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE,
    haystack    TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS clipmem_settings (
    id                INTEGER PRIMARY KEY CHECK (id = 1),
    paused            INTEGER NOT NULL DEFAULT 0 CHECK (paused IN (0, 1)),
    retention_seconds INTEGER CHECK (retention_seconds IS NULL OR retention_seconds >= 0),
    api_key_filter_enabled INTEGER NOT NULL DEFAULT 0 CHECK (api_key_filter_enabled IN (0, 1)),
    ocr_enabled       INTEGER NOT NULL DEFAULT 0 CHECK (ocr_enabled IN (0, 1)),
    representation_cache_deferred INTEGER NOT NULL DEFAULT 0 CHECK (representation_cache_deferred IN (0, 1))
);

CREATE TABLE IF NOT EXISTS archive_revisions (
    id                         INTEGER PRIMARY KEY CHECK (id = 1),
    revision                   INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
    archive_content_revision   INTEGER NOT NULL DEFAULT 0 CHECK (archive_content_revision >= 0),
    settings_revision          INTEGER NOT NULL DEFAULT 0 CHECK (settings_revision >= 0),
    ocr_revision               INTEGER NOT NULL DEFAULT 0 CHECK (ocr_revision >= 0),
    storage_revision           INTEGER NOT NULL DEFAULT 0 CHECK (storage_revision >= 0),
    service_revision           INTEGER NOT NULL DEFAULT 0 CHECK (service_revision >= 0),
    app_preferences_revision   INTEGER NOT NULL DEFAULT 0 CHECK (app_preferences_revision >= 0),
    last_change_kind           TEXT NOT NULL DEFAULT 'initialized',
    updated_at                 TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ignored_bundle_ids (
    bundle_id TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS pending_restores (
    snapshot_sha256 TEXT PRIMARY KEY,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ocr_results (
    raw_sha256        TEXT PRIMARY KEY,
    status            TEXT NOT NULL CHECK (status IN ('pending', 'ready', 'failed', 'skipped')),
    engine            TEXT,
    recognition_level TEXT,
    text_value        TEXT,
    error             TEXT,
    attempt_count     INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    created_at        TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at        TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS snapshot_ocr_cache (
    snapshot_id INTEGER PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE,
    ocr_text    TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'skipped' CHECK (status IN ('pending', 'ready', 'failed', 'skipped')),
    updated_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_capture_events_snapshot_id
    ON capture_events(snapshot_id);

CREATE INDEX IF NOT EXISTS idx_capture_events_snapshot_observed_id
    ON capture_events(snapshot_id, observed_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_capture_events_observed_id
    ON capture_events(observed_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_capture_events_app_group
    ON capture_events(COALESCE(NULLIF(frontmost_app_name, ''), NULLIF(frontmost_app_bundle_id, ''), 'Unknown'));

CREATE INDEX IF NOT EXISTS idx_capture_events_observed_hour
    ON capture_events(CAST(strftime('%H', observed_at) AS INTEGER));

CREATE INDEX IF NOT EXISTS idx_capture_events_observed_weekday
    ON capture_events(CAST(strftime('%w', observed_at) AS INTEGER));

CREATE INDEX IF NOT EXISTS idx_snapshot_stats_last_observed_snapshot
    ON snapshot_stats(last_observed_at DESC, snapshot_id DESC);

CREATE INDEX IF NOT EXISTS idx_snapshots_total_bytes
    ON snapshots(total_bytes DESC, id ASC);

CREATE INDEX IF NOT EXISTS idx_snapshot_stats_capture_count
    ON snapshot_stats(capture_count DESC, snapshot_id ASC);

CREATE INDEX IF NOT EXISTS idx_pending_restores_created_at
    ON pending_restores(created_at);

CREATE INDEX IF NOT EXISTS idx_snapshot_items_snapshot_id
    ON snapshot_items(snapshot_id, item_index);

CREATE INDEX IF NOT EXISTS idx_item_representations_image_candidates
    ON item_representations(kind, raw_sha256)
    WHERE length(blob_value) > 0;

CREATE INDEX IF NOT EXISTS idx_item_representations_raw_sha256_snapshot
    ON item_representations(raw_sha256, snapshot_id);

CREATE INDEX IF NOT EXISTS idx_ocr_results_status
    ON ocr_results(status);

CREATE INDEX IF NOT EXISTS idx_ocr_results_pending_queue
    ON ocr_results(status, updated_at ASC, raw_sha256 ASC);

CREATE INDEX IF NOT EXISTS idx_snapshot_ocr_cache_text_present
    ON snapshot_ocr_cache(snapshot_id)
    WHERE ocr_text != '';

CREATE VIRTUAL TABLE IF NOT EXISTS snapshots_fts USING fts5(
    search_text,
    preview_text,
    content='snapshots',
    content_rowid='id',
    tokenize='unicode61'
);

CREATE VIRTUAL TABLE IF NOT EXISTS snapshots_literal_fts USING fts5(
    haystack,
    tokenize='trigram'
);

CREATE VIRTUAL TABLE IF NOT EXISTS snapshot_file_url_fts USING fts5(
    file_urls,
    content='snapshot_projection_cache',
    content_rowid='snapshot_id',
    tokenize='trigram'
);

CREATE VIRTUAL TABLE IF NOT EXISTS snapshot_ocr_fts USING fts5(
    ocr_text,
    content='snapshot_ocr_cache',
    content_rowid='snapshot_id',
    tokenize='unicode61'
);

CREATE VIRTUAL TABLE IF NOT EXISTS snapshot_ocr_literal_fts USING fts5(
    ocr_text,
    tokenize='trigram'
);

CREATE TRIGGER IF NOT EXISTS snapshots_ai AFTER INSERT ON snapshots BEGIN
    INSERT INTO snapshots_fts(rowid, search_text, preview_text)
    VALUES (new.id, new.search_text, new.preview_text);
    INSERT INTO snapshot_literal_cache (snapshot_id, haystack)
    VALUES (
        new.id,
        lower(
            COALESCE(NULLIF(new.preview_text, ''), new.search_text, '') || char(31) ||
            COALESCE(new.preview_text, '') || char(31) ||
            COALESCE(new.search_text, '')
        )
    )
    ON CONFLICT(snapshot_id) DO UPDATE SET
        haystack = excluded.haystack;
END;

CREATE TRIGGER IF NOT EXISTS snapshots_ad AFTER DELETE ON snapshots BEGIN
    INSERT INTO snapshots_fts(snapshots_fts, rowid, search_text, preview_text)
    VALUES ('delete', old.id, old.search_text, old.preview_text);
END;

CREATE TRIGGER IF NOT EXISTS snapshots_au AFTER UPDATE ON snapshots BEGIN
    INSERT INTO snapshots_fts(snapshots_fts, rowid, search_text, preview_text)
    VALUES ('delete', old.id, old.search_text, old.preview_text);
    INSERT INTO snapshots_fts(rowid, search_text, preview_text)
    VALUES (new.id, new.search_text, new.preview_text);
    INSERT INTO snapshot_literal_cache (snapshot_id, haystack)
    VALUES (
        new.id,
        lower(
            COALESCE(NULLIF(new.preview_text, ''), new.search_text, '') || char(31) ||
            COALESCE(new.preview_text, '') || char(31) ||
            COALESCE(new.search_text, '')
        )
    )
    ON CONFLICT(snapshot_id) DO UPDATE SET
        haystack = excluded.haystack;
END;

CREATE TRIGGER IF NOT EXISTS snapshot_literal_cache_ai
AFTER INSERT ON snapshot_literal_cache BEGIN
    INSERT INTO snapshots_literal_fts(rowid, haystack)
    VALUES (new.snapshot_id, new.haystack);
END;

CREATE TRIGGER IF NOT EXISTS snapshot_literal_cache_au
AFTER UPDATE ON snapshot_literal_cache BEGIN
    DELETE FROM snapshots_literal_fts WHERE rowid = old.snapshot_id;
    INSERT INTO snapshots_literal_fts(rowid, haystack)
    VALUES (new.snapshot_id, new.haystack);
END;

CREATE TRIGGER IF NOT EXISTS snapshot_literal_cache_ad
AFTER DELETE ON snapshot_literal_cache BEGIN
    DELETE FROM snapshots_literal_fts WHERE rowid = old.snapshot_id;
END;

CREATE TRIGGER IF NOT EXISTS snapshot_projection_cache_ai
AFTER INSERT ON snapshot_projection_cache BEGIN
    INSERT INTO snapshot_file_url_fts(rowid, file_urls)
    VALUES (new.snapshot_id, new.file_urls);
END;

CREATE TRIGGER IF NOT EXISTS snapshot_projection_cache_au
AFTER UPDATE ON snapshot_projection_cache BEGIN
    INSERT INTO snapshot_file_url_fts(snapshot_file_url_fts, rowid, file_urls)
    VALUES ('delete', old.snapshot_id, old.file_urls);
    INSERT INTO snapshot_file_url_fts(rowid, file_urls)
    VALUES (new.snapshot_id, new.file_urls);
END;

CREATE TRIGGER IF NOT EXISTS snapshot_projection_cache_ad
AFTER DELETE ON snapshot_projection_cache BEGIN
    INSERT INTO snapshot_file_url_fts(snapshot_file_url_fts, rowid, file_urls)
    VALUES ('delete', old.snapshot_id, old.file_urls);
END;

CREATE TRIGGER IF NOT EXISTS snapshot_ocr_cache_ai
AFTER INSERT ON snapshot_ocr_cache BEGIN
    INSERT INTO snapshot_ocr_fts(rowid, ocr_text)
    VALUES (new.snapshot_id, new.ocr_text);
    INSERT INTO snapshot_ocr_literal_fts(rowid, ocr_text)
    VALUES (new.snapshot_id, lower(new.ocr_text));
END;

CREATE TRIGGER IF NOT EXISTS snapshot_ocr_cache_au
AFTER UPDATE ON snapshot_ocr_cache BEGIN
    INSERT INTO snapshot_ocr_fts(snapshot_ocr_fts, rowid, ocr_text)
    VALUES ('delete', old.snapshot_id, old.ocr_text);
    INSERT INTO snapshot_ocr_fts(rowid, ocr_text)
    VALUES (new.snapshot_id, new.ocr_text);
    DELETE FROM snapshot_ocr_literal_fts WHERE rowid = old.snapshot_id;
    INSERT INTO snapshot_ocr_literal_fts(rowid, ocr_text)
    VALUES (new.snapshot_id, lower(new.ocr_text));
END;

CREATE TRIGGER IF NOT EXISTS snapshot_ocr_cache_ad
AFTER DELETE ON snapshot_ocr_cache BEGIN
    INSERT INTO snapshot_ocr_fts(snapshot_ocr_fts, rowid, ocr_text)
    VALUES ('delete', old.snapshot_id, old.ocr_text);
    DELETE FROM snapshot_ocr_literal_fts WHERE rowid = old.snapshot_id;
END;

CREATE TRIGGER IF NOT EXISTS capture_events_restore_suppression_bi
BEFORE INSERT ON capture_events BEGIN
    DELETE FROM pending_restores
    WHERE datetime(created_at) < datetime('now', '-30 seconds');
    DELETE FROM pending_restores
    WHERE snapshot_sha256 = (SELECT sha256 FROM snapshots WHERE id = new.snapshot_id)
      AND datetime(created_at) >= datetime('now', '-30 seconds');
    SELECT CASE WHEN changes() > 0 THEN RAISE(IGNORE) END;
END;

DROP TRIGGER IF EXISTS capture_events_ai;
CREATE TRIGGER capture_events_ai AFTER INSERT ON capture_events BEGIN
    INSERT INTO snapshot_stats (
        snapshot_id,
        capture_count,
        first_observed_at,
        last_observed_at,
        last_event_id,
        last_frontmost_app_bundle_id,
        last_frontmost_app_name
    ) VALUES (
        new.snapshot_id,
        1,
        new.observed_at,
        new.observed_at,
        new.id,
        new.frontmost_app_bundle_id,
        new.frontmost_app_name
    )
    ON CONFLICT(snapshot_id) DO UPDATE SET
        capture_count = snapshot_stats.capture_count + 1,
        first_observed_at = MIN(snapshot_stats.first_observed_at, new.observed_at),
        last_observed_at = CASE
            WHEN new.observed_at > snapshot_stats.last_observed_at
                OR (
                    new.observed_at = snapshot_stats.last_observed_at
                    AND new.id > snapshot_stats.last_event_id
                )
                THEN new.observed_at
            ELSE snapshot_stats.last_observed_at
        END,
        last_event_id = CASE
            WHEN new.observed_at > snapshot_stats.last_observed_at
                OR (
                    new.observed_at = snapshot_stats.last_observed_at
                    AND new.id > snapshot_stats.last_event_id
                )
                THEN new.id
            ELSE snapshot_stats.last_event_id
        END,
        last_frontmost_app_bundle_id = CASE
            WHEN new.observed_at > snapshot_stats.last_observed_at
                OR (
                    new.observed_at = snapshot_stats.last_observed_at
                    AND new.id > snapshot_stats.last_event_id
                )
                THEN new.frontmost_app_bundle_id
            ELSE snapshot_stats.last_frontmost_app_bundle_id
        END,
        last_frontmost_app_name = CASE
            WHEN new.observed_at > snapshot_stats.last_observed_at
                OR (
                    new.observed_at = snapshot_stats.last_observed_at
                    AND new.id > snapshot_stats.last_event_id
                )
                THEN new.frontmost_app_name
            ELSE snapshot_stats.last_frontmost_app_name
        END;
    INSERT INTO snapshot_event_filter_cache (
        snapshot_id,
        app_names_lower,
        bundle_ids_lower
    ) VALUES (
        new.snapshot_id,
        COALESCE(lower(new.frontmost_app_name), ''),
        COALESCE(lower(new.frontmost_app_bundle_id), '')
    )
    ON CONFLICT(snapshot_id) DO UPDATE SET
        app_names_lower = CASE
            WHEN excluded.app_names_lower = ''
                THEN snapshot_event_filter_cache.app_names_lower
            WHEN snapshot_event_filter_cache.app_names_lower = ''
                THEN excluded.app_names_lower
            ELSE COALESCE((
                SELECT GROUP_CONCAT(app_name, char(31))
                FROM (
                    SELECT DISTINCT lower(frontmost_app_name) AS app_name
                    FROM capture_events
                    WHERE snapshot_id = new.snapshot_id
                      AND frontmost_app_name IS NOT NULL
                      AND frontmost_app_name != ''
                    ORDER BY app_name
                )
            ), '')
        END,
        bundle_ids_lower = CASE
            WHEN excluded.bundle_ids_lower = ''
                THEN snapshot_event_filter_cache.bundle_ids_lower
            WHEN snapshot_event_filter_cache.bundle_ids_lower = ''
                THEN excluded.bundle_ids_lower
            ELSE COALESCE((
                SELECT GROUP_CONCAT(bundle_id, char(31))
                FROM (
                    SELECT DISTINCT lower(frontmost_app_bundle_id) AS bundle_id
                    FROM capture_events
                    WHERE snapshot_id = new.snapshot_id
                      AND frontmost_app_bundle_id IS NOT NULL
                      AND frontmost_app_bundle_id != ''
                    ORDER BY bundle_id
                )
            ), '')
        END
    WHERE (
        excluded.app_names_lower != ''
        AND instr(
            char(31) || snapshot_event_filter_cache.app_names_lower || char(31),
            char(31) || excluded.app_names_lower || char(31)
        ) = 0
    ) OR (
        excluded.bundle_ids_lower != ''
        AND instr(
            char(31) || snapshot_event_filter_cache.bundle_ids_lower || char(31),
            char(31) || excluded.bundle_ids_lower || char(31)
        ) = 0
    );
    INSERT INTO snapshot_literal_cache (snapshot_id, haystack)
    SELECT
        s.id,
        lower(
            COALESCE(NULLIF(s.preview_text, ''), s.search_text, '') || char(31) ||
            COALESCE(s.preview_text, '') || char(31) ||
            COALESCE(s.search_text, '') || char(31) ||
            COALESCE(sp.urls, '') || char(31) ||
            COALESCE(sp.file_urls, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_name, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_bundle_id, '')
        )
    FROM snapshots s
    LEFT JOIN snapshot_projection_cache sp ON sp.snapshot_id = s.id
    LEFT JOIN snapshot_stats ss ON ss.snapshot_id = s.id
    WHERE s.id = new.snapshot_id
    ON CONFLICT(snapshot_id) DO UPDATE SET
        haystack = excluded.haystack
    WHERE snapshot_literal_cache.haystack IS NOT excluded.haystack;
END;

CREATE TRIGGER IF NOT EXISTS capture_events_au
AFTER UPDATE OF observed_at, frontmost_app_bundle_id, frontmost_app_name ON capture_events BEGIN
    DELETE FROM snapshot_stats WHERE snapshot_id = old.snapshot_id;
    INSERT INTO snapshot_stats (
        snapshot_id,
        capture_count,
        first_observed_at,
        last_observed_at,
        last_event_id,
        last_frontmost_app_bundle_id,
        last_frontmost_app_name
    )
    SELECT
        ce.snapshot_id,
        COUNT(*) AS capture_count,
        MIN(ce.observed_at) AS first_observed_at,
        MAX(ce.observed_at) AS last_observed_at,
        (
            SELECT latest.id
            FROM capture_events latest
            WHERE latest.snapshot_id = ce.snapshot_id
            ORDER BY latest.observed_at DESC, latest.id DESC
            LIMIT 1
        ) AS last_event_id,
        (
            SELECT latest.frontmost_app_bundle_id
            FROM capture_events latest
            WHERE latest.snapshot_id = ce.snapshot_id
            ORDER BY latest.observed_at DESC, latest.id DESC
            LIMIT 1
        ) AS last_frontmost_app_bundle_id,
        (
            SELECT latest.frontmost_app_name
            FROM capture_events latest
            WHERE latest.snapshot_id = ce.snapshot_id
            ORDER BY latest.observed_at DESC, latest.id DESC
            LIMIT 1
        ) AS last_frontmost_app_name
    FROM capture_events ce
    WHERE ce.snapshot_id = new.snapshot_id
    GROUP BY ce.snapshot_id;
    DELETE FROM snapshot_event_filter_cache WHERE snapshot_id = old.snapshot_id;
    INSERT INTO snapshot_event_filter_cache (
        snapshot_id,
        app_names_lower,
        bundle_ids_lower
    )
    SELECT
        ce.snapshot_id,
        COALESCE((
            SELECT GROUP_CONCAT(app_name, char(31))
            FROM (
                SELECT DISTINCT lower(latest.frontmost_app_name) AS app_name
                FROM capture_events latest
                WHERE latest.snapshot_id = ce.snapshot_id
                  AND latest.frontmost_app_name IS NOT NULL
                  AND latest.frontmost_app_name != ''
                ORDER BY app_name
            )
        ), '') AS app_names_lower,
        COALESCE((
            SELECT GROUP_CONCAT(bundle_id, char(31))
            FROM (
                SELECT DISTINCT lower(latest.frontmost_app_bundle_id) AS bundle_id
                FROM capture_events latest
                WHERE latest.snapshot_id = ce.snapshot_id
                  AND latest.frontmost_app_bundle_id IS NOT NULL
                  AND latest.frontmost_app_bundle_id != ''
                ORDER BY bundle_id
            )
        ), '') AS bundle_ids_lower
    FROM capture_events ce
    WHERE ce.snapshot_id = new.snapshot_id
    GROUP BY ce.snapshot_id;
    INSERT INTO snapshot_literal_cache (snapshot_id, haystack)
    SELECT
        s.id,
        lower(
            COALESCE(NULLIF(s.preview_text, ''), s.search_text, '') || char(31) ||
            COALESCE(s.preview_text, '') || char(31) ||
            COALESCE(s.search_text, '') || char(31) ||
            COALESCE(sp.urls, '') || char(31) ||
            COALESCE(sp.file_urls, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_name, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_bundle_id, '')
        )
    FROM snapshots s
    LEFT JOIN snapshot_projection_cache sp ON sp.snapshot_id = s.id
    LEFT JOIN snapshot_stats ss ON ss.snapshot_id = s.id
    WHERE s.id = new.snapshot_id
    ON CONFLICT(snapshot_id) DO UPDATE SET
        haystack = excluded.haystack;
END;

CREATE TRIGGER IF NOT EXISTS capture_events_ad AFTER DELETE ON capture_events BEGIN
    DELETE FROM snapshot_stats WHERE snapshot_id = old.snapshot_id;
    INSERT INTO snapshot_stats (
        snapshot_id,
        capture_count,
        first_observed_at,
        last_observed_at,
        last_event_id,
        last_frontmost_app_bundle_id,
        last_frontmost_app_name
    )
    SELECT
        ce.snapshot_id,
        COUNT(*) AS capture_count,
        MIN(ce.observed_at) AS first_observed_at,
        MAX(ce.observed_at) AS last_observed_at,
        (
            SELECT latest.id
            FROM capture_events latest
            WHERE latest.snapshot_id = ce.snapshot_id
            ORDER BY latest.observed_at DESC, latest.id DESC
            LIMIT 1
        ) AS last_event_id,
        (
            SELECT latest.frontmost_app_bundle_id
            FROM capture_events latest
            WHERE latest.snapshot_id = ce.snapshot_id
            ORDER BY latest.observed_at DESC, latest.id DESC
            LIMIT 1
        ) AS last_frontmost_app_bundle_id,
        (
            SELECT latest.frontmost_app_name
            FROM capture_events latest
            WHERE latest.snapshot_id = ce.snapshot_id
            ORDER BY latest.observed_at DESC, latest.id DESC
            LIMIT 1
        ) AS last_frontmost_app_name
    FROM capture_events ce
    WHERE ce.snapshot_id = old.snapshot_id
    GROUP BY ce.snapshot_id;
    DELETE FROM snapshot_event_filter_cache WHERE snapshot_id = old.snapshot_id;
    INSERT INTO snapshot_event_filter_cache (
        snapshot_id,
        app_names_lower,
        bundle_ids_lower
    )
    SELECT
        ce.snapshot_id,
        COALESCE((
            SELECT GROUP_CONCAT(app_name, char(31))
            FROM (
                SELECT DISTINCT lower(latest.frontmost_app_name) AS app_name
                FROM capture_events latest
                WHERE latest.snapshot_id = ce.snapshot_id
                  AND latest.frontmost_app_name IS NOT NULL
                  AND latest.frontmost_app_name != ''
                ORDER BY app_name
            )
        ), '') AS app_names_lower,
        COALESCE((
            SELECT GROUP_CONCAT(bundle_id, char(31))
            FROM (
                SELECT DISTINCT lower(latest.frontmost_app_bundle_id) AS bundle_id
                FROM capture_events latest
                WHERE latest.snapshot_id = ce.snapshot_id
                  AND latest.frontmost_app_bundle_id IS NOT NULL
                  AND latest.frontmost_app_bundle_id != ''
                ORDER BY bundle_id
            )
        ), '') AS bundle_ids_lower
    FROM capture_events ce
    WHERE ce.snapshot_id = old.snapshot_id
    GROUP BY ce.snapshot_id;
    INSERT INTO snapshot_literal_cache (snapshot_id, haystack)
    SELECT
        s.id,
        lower(
            COALESCE(NULLIF(s.preview_text, ''), s.search_text, '') || char(31) ||
            COALESCE(s.preview_text, '') || char(31) ||
            COALESCE(s.search_text, '') || char(31) ||
            COALESCE(sp.urls, '') || char(31) ||
            COALESCE(sp.file_urls, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_name, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_bundle_id, '')
        )
    FROM snapshots s
    LEFT JOIN snapshot_projection_cache sp ON sp.snapshot_id = s.id
    LEFT JOIN snapshot_stats ss ON ss.snapshot_id = s.id
    WHERE s.id = old.snapshot_id
    ON CONFLICT(snapshot_id) DO UPDATE SET
        haystack = excluded.haystack;
END;

DROP TRIGGER IF EXISTS item_representations_ai;
CREATE TRIGGER item_representations_ai AFTER INSERT ON item_representations
WHEN NOT EXISTS (
    SELECT 1 FROM clipmem_settings
    WHERE id = 1 AND representation_cache_deferred = 1
)
BEGIN
    INSERT INTO snapshot_projection_cache (snapshot_id, urls, file_urls)
    VALUES (new.snapshot_id, '', '')
    ON CONFLICT(snapshot_id) DO NOTHING;
    UPDATE snapshot_projection_cache
    SET
        urls = COALESCE((
            SELECT GROUP_CONCAT(text_value, char(31))
            FROM (
                SELECT DISTINCT text_value
                FROM item_representations
                WHERE snapshot_id = new.snapshot_id
                  AND kind = 'url'
                  AND text_value IS NOT NULL
                  AND text_value != ''
                ORDER BY text_value
            )
        ), ''),
        file_urls = COALESCE((
            SELECT GROUP_CONCAT(text_value, char(31))
            FROM (
                SELECT DISTINCT text_value
                FROM item_representations
                WHERE snapshot_id = new.snapshot_id
                  AND kind = 'file_url'
                  AND text_value IS NOT NULL
                  AND text_value != ''
                ORDER BY text_value
            )
        ), '')
    WHERE snapshot_id = new.snapshot_id;
    INSERT INTO snapshot_literal_cache (snapshot_id, haystack)
    SELECT
        s.id,
        lower(
            COALESCE(NULLIF(s.preview_text, ''), s.search_text, '') || char(31) ||
            COALESCE(s.preview_text, '') || char(31) ||
            COALESCE(s.search_text, '') || char(31) ||
            COALESCE(sp.urls, '') || char(31) ||
            COALESCE(sp.file_urls, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_name, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_bundle_id, '')
        )
    FROM snapshots s
    LEFT JOIN snapshot_projection_cache sp ON sp.snapshot_id = s.id
    LEFT JOIN snapshot_stats ss ON ss.snapshot_id = s.id
    WHERE s.id = new.snapshot_id
    ON CONFLICT(snapshot_id) DO UPDATE SET
        haystack = excluded.haystack;
END;

DROP TRIGGER IF EXISTS item_representations_au;
CREATE TRIGGER item_representations_au AFTER UPDATE ON item_representations
WHEN NOT EXISTS (
    SELECT 1 FROM clipmem_settings
    WHERE id = 1 AND representation_cache_deferred = 1
)
BEGIN
    INSERT INTO snapshot_projection_cache (snapshot_id, urls, file_urls)
    VALUES (new.snapshot_id, '', '')
    ON CONFLICT(snapshot_id) DO NOTHING;
    UPDATE snapshot_projection_cache
    SET
        urls = COALESCE((
            SELECT GROUP_CONCAT(text_value, char(31))
            FROM (
                SELECT DISTINCT text_value
                FROM item_representations
                WHERE snapshot_id = old.snapshot_id
                  AND kind = 'url'
                  AND text_value IS NOT NULL
                  AND text_value != ''
                ORDER BY text_value
            )
        ), ''),
        file_urls = COALESCE((
            SELECT GROUP_CONCAT(text_value, char(31))
            FROM (
                SELECT DISTINCT text_value
                FROM item_representations
                WHERE snapshot_id = old.snapshot_id
                  AND kind = 'file_url'
                  AND text_value IS NOT NULL
                  AND text_value != ''
                ORDER BY text_value
            )
        ), '')
    WHERE snapshot_id = old.snapshot_id;
    INSERT INTO snapshot_literal_cache (snapshot_id, haystack)
    SELECT
        s.id,
        lower(
            COALESCE(NULLIF(s.preview_text, ''), s.search_text, '') || char(31) ||
            COALESCE(s.preview_text, '') || char(31) ||
            COALESCE(s.search_text, '') || char(31) ||
            COALESCE(sp.urls, '') || char(31) ||
            COALESCE(sp.file_urls, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_name, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_bundle_id, '')
        )
    FROM snapshots s
    LEFT JOIN snapshot_projection_cache sp ON sp.snapshot_id = s.id
    LEFT JOIN snapshot_stats ss ON ss.snapshot_id = s.id
    WHERE s.id = old.snapshot_id
    ON CONFLICT(snapshot_id) DO UPDATE SET
        haystack = excluded.haystack;
    UPDATE snapshot_projection_cache
    SET
        urls = COALESCE((
            SELECT GROUP_CONCAT(text_value, char(31))
            FROM (
                SELECT DISTINCT text_value
                FROM item_representations
                WHERE snapshot_id = new.snapshot_id
                  AND kind = 'url'
                  AND text_value IS NOT NULL
                  AND text_value != ''
                ORDER BY text_value
            )
        ), ''),
        file_urls = COALESCE((
            SELECT GROUP_CONCAT(text_value, char(31))
            FROM (
                SELECT DISTINCT text_value
                FROM item_representations
                WHERE snapshot_id = new.snapshot_id
                  AND kind = 'file_url'
                  AND text_value IS NOT NULL
                  AND text_value != ''
                ORDER BY text_value
            )
        ), '')
    WHERE snapshot_id = new.snapshot_id;
    INSERT INTO snapshot_literal_cache (snapshot_id, haystack)
    SELECT
        s.id,
        lower(
            COALESCE(NULLIF(s.preview_text, ''), s.search_text, '') || char(31) ||
            COALESCE(s.preview_text, '') || char(31) ||
            COALESCE(s.search_text, '') || char(31) ||
            COALESCE(sp.urls, '') || char(31) ||
            COALESCE(sp.file_urls, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_name, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_bundle_id, '')
        )
    FROM snapshots s
    LEFT JOIN snapshot_projection_cache sp ON sp.snapshot_id = s.id
    LEFT JOIN snapshot_stats ss ON ss.snapshot_id = s.id
    WHERE s.id = new.snapshot_id
    ON CONFLICT(snapshot_id) DO UPDATE SET
        haystack = excluded.haystack;
END;

DROP TRIGGER IF EXISTS item_representations_ad;
CREATE TRIGGER item_representations_ad AFTER DELETE ON item_representations
WHEN NOT EXISTS (
    SELECT 1 FROM clipmem_settings
    WHERE id = 1 AND representation_cache_deferred = 1
)
BEGIN
    UPDATE snapshot_projection_cache
    SET
        urls = COALESCE((
            SELECT GROUP_CONCAT(text_value, char(31))
            FROM (
                SELECT DISTINCT text_value
                FROM item_representations
                WHERE snapshot_id = old.snapshot_id
                  AND kind = 'url'
                  AND text_value IS NOT NULL
                  AND text_value != ''
                ORDER BY text_value
            )
        ), ''),
        file_urls = COALESCE((
            SELECT GROUP_CONCAT(text_value, char(31))
            FROM (
                SELECT DISTINCT text_value
                FROM item_representations
                WHERE snapshot_id = old.snapshot_id
                  AND kind = 'file_url'
                  AND text_value IS NOT NULL
                  AND text_value != ''
                ORDER BY text_value
            )
        ), '')
    WHERE snapshot_id = old.snapshot_id;
    INSERT INTO snapshot_literal_cache (snapshot_id, haystack)
    SELECT
        s.id,
        lower(
            COALESCE(NULLIF(s.preview_text, ''), s.search_text, '') || char(31) ||
            COALESCE(s.preview_text, '') || char(31) ||
            COALESCE(s.search_text, '') || char(31) ||
            COALESCE(sp.urls, '') || char(31) ||
            COALESCE(sp.file_urls, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_name, '') || char(31) ||
            COALESCE(ss.last_frontmost_app_bundle_id, '')
        )
    FROM snapshots s
    LEFT JOIN snapshot_projection_cache sp ON sp.snapshot_id = s.id
    LEFT JOIN snapshot_stats ss ON ss.snapshot_id = s.id
    WHERE s.id = old.snapshot_id
    ON CONFLICT(snapshot_id) DO UPDATE SET
        haystack = excluded.haystack;
END;
