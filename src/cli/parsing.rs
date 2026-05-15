use anyhow::Result;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::db::{RetrievalKind, SearchMode, TimelineSort};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DurationValue {
    pub(in crate::cli) raw: String,
    pub(in crate::cli) seconds: u64,
}

impl DurationValue {
    #[must_use]
    pub(super) fn new(raw: String, seconds: u64) -> Self {
        Self { raw, seconds }
    }

    #[must_use]
    pub(super) fn raw(&self) -> &str {
        &self.raw
    }

    #[must_use]
    pub(super) fn seconds(&self) -> u64 {
        self.seconds
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RetentionValue {
    Forever,
    Duration(DurationValue),
}

impl RetentionValue {
    #[must_use]
    pub(super) fn retention_seconds(&self) -> Option<u64> {
        match self {
            Self::Forever => None,
            Self::Duration(duration) => Some(duration.seconds()),
        }
    }
}

pub(super) fn parse_normalized_score(value: &str) -> Result<f64, LimitParseError> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| LimitParseError(format!("invalid floating-point value '{value}'")))?;

    if (0.0..=1.0).contains(&parsed) {
        Ok(parsed)
    } else {
        Err(LimitParseError(format!(
            "value must be between 0.0 and 1.0, got {parsed}"
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LimitParseError(String);

impl std::fmt::Display for LimitParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for LimitParseError {}

pub(super) fn parse_bounded_limit(value: &str) -> Result<usize, LimitParseError> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| LimitParseError(format!("invalid integer value '{value}'")))?;

    if (1..=250).contains(&parsed) {
        Ok(parsed)
    } else {
        Err(LimitParseError(format!(
            "value must be between 1 and 250, got {parsed}"
        )))
    }
}

pub(super) fn parse_rfc3339_timestamp(value: &str) -> Result<String, LimitParseError> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| LimitParseError(format!("invalid RFC3339 timestamp '{value}'")))
        .and_then(|timestamp| {
            timestamp
                .format(&Rfc3339)
                .map_err(|error| LimitParseError(format!("format timestamp '{value}': {error}")))
        })
}

pub(super) fn parse_nonnegative_bytes(value: &str) -> Result<usize, LimitParseError> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| LimitParseError(format!("invalid integer value '{value}'")))?;

    if parsed <= i64::MAX as usize {
        Ok(parsed)
    } else {
        Err(LimitParseError(format!(
            "byte value '{value}' exceeds supported range"
        )))
    }
}

pub(super) fn parse_item_index(value: &str) -> Result<usize, LimitParseError> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| LimitParseError(format!("invalid item index '{value}'")))?;

    if parsed <= i64::MAX as usize {
        Ok(parsed)
    } else {
        Err(LimitParseError(format!(
            "item index '{value}' exceeds supported range"
        )))
    }
}

pub(super) fn parse_sha256_hash(value: &str) -> Result<String, LimitParseError> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(value.to_ascii_lowercase())
    } else {
        Err(LimitParseError(format!(
            "invalid SHA-256 hash '{value}'; expected 64 hexadecimal characters"
        )))
    }
}

pub(super) fn parse_bundle_id(value: &str) -> Result<String, LimitParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(LimitParseError("bundle id cannot be empty".to_string()))
    } else {
        Ok(trimmed.to_string())
    }
}

pub(super) fn parse_preferred_app(value: &str) -> Result<String, LimitParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(LimitParseError("preferred app cannot be empty".to_string()))
    } else {
        Ok(trimmed.to_string())
    }
}

pub(super) fn parse_search_mode(value: &str) -> Result<SearchMode, LimitParseError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(SearchMode::Auto),
        "fts" => Ok(SearchMode::Fts),
        "literal" => Ok(SearchMode::Literal),
        _ => Err(one_of_error("search mode", value, "auto, fts, or literal")),
    }
}

pub(super) fn parse_timeline_sort(value: &str) -> Result<TimelineSort, LimitParseError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "asc" => Ok(TimelineSort::Asc),
        "desc" => Ok(TimelineSort::Desc),
        _ => Err(one_of_error("timeline sort", value, "asc or desc")),
    }
}

pub(super) fn parse_retrieval_kind(value: &str) -> Result<RetrievalKind, LimitParseError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "text" => Ok(RetrievalKind::Text),
        "html" => Ok(RetrievalKind::Html),
        "rtf" => Ok(RetrievalKind::Rtf),
        "url" => Ok(RetrievalKind::Url),
        "file" => Ok(RetrievalKind::File),
        "image" => Ok(RetrievalKind::Image),
        "pdf" => Ok(RetrievalKind::Pdf),
        "binary" => Ok(RetrievalKind::Binary),
        "other" => Ok(RetrievalKind::Other),
        _ => Err(one_of_error(
            "retrieval kind",
            value,
            "text, html, rtf, url, file, image, pdf, binary, or other",
        )),
    }
}

fn one_of_error(label: &str, value: &str, allowed: &str) -> LimitParseError {
    LimitParseError(format!("invalid {label} '{value}'; expected {allowed}"))
}

pub(super) fn parse_duration_value(value: &str) -> Result<DurationValue, LimitParseError> {
    let trimmed = value.trim();
    if trimmed.len() < 2 {
        return Err(LimitParseError(format!(
            "invalid duration '{value}'; expected <integer><unit> like 30d, 12h, or 15m"
        )));
    }

    let (amount, unit) = trimmed.split_at(trimmed.len() - 1);
    let amount = amount.parse::<u64>().map_err(|_| {
        LimitParseError(format!(
            "invalid duration '{value}'; expected an integer amount before the unit"
        ))
    })?;
    if amount == 0 {
        return Err(LimitParseError(
            "duration must be greater than zero".to_string(),
        ));
    }

    let seconds_per_unit = match unit.to_ascii_lowercase().as_str() {
        "d" => 24 * 60 * 60,
        "h" => 60 * 60,
        "m" => 60,
        _ => {
            return Err(LimitParseError(format!(
                "invalid duration unit '{unit}'; expected d, h, or m"
            )))
        }
    };

    let seconds = amount
        .checked_mul(seconds_per_unit)
        .filter(|seconds| *seconds <= i64::MAX as u64)
        .ok_or_else(|| LimitParseError(format!("duration '{value}' exceeds supported range")))?;

    Ok(DurationValue::new(trimmed.to_string(), seconds))
}

pub(super) fn parse_retention_value(value: &str) -> Result<RetentionValue, LimitParseError> {
    if value.trim().eq_ignore_ascii_case("forever") {
        Ok(RetentionValue::Forever)
    } else {
        parse_duration_value(value).map(RetentionValue::Duration)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::{RetrievalKind, SearchMode, TimelineSort};

    use super::{
        parse_bundle_id, parse_retrieval_kind, parse_search_mode, parse_sha256_hash,
        parse_timeline_sort, DurationValue, RetentionValue,
    };

    #[test]
    fn retention_value_returns_none_for_forever_and_seconds_for_duration() {
        assert_eq!(RetentionValue::Forever.retention_seconds(), None);
        assert_eq!(
            RetentionValue::Duration(DurationValue::new("2h".to_string(), 7_200))
                .retention_seconds(),
            Some(7_200)
        );
    }

    #[test]
    fn cli_domain_parsers_return_database_enums() {
        assert_eq!(parse_search_mode("literal").unwrap(), SearchMode::Literal);
        assert_eq!(parse_timeline_sort("asc").unwrap(), TimelineSort::Asc);
        assert_eq!(parse_retrieval_kind("file").unwrap(), RetrievalKind::File);
        assert!(parse_search_mode("regex").is_err());
        assert!(parse_timeline_sort("newest").is_err());
        assert!(parse_retrieval_kind("folder").is_err());
    }

    #[test]
    fn sha256_hash_parser_accepts_hex_and_normalizes_case() {
        let uppercase = "A".repeat(64);
        assert_eq!(parse_sha256_hash(&uppercase).unwrap(), "a".repeat(64));

        assert!(parse_sha256_hash("abc123").is_err());
        assert!(parse_sha256_hash(&"g".repeat(64)).is_err());
    }

    #[test]
    fn bundle_id_parser_trims_and_rejects_empty_values() {
        assert_eq!(
            parse_bundle_id(" Com.Apple.Terminal ").unwrap(),
            "Com.Apple.Terminal"
        );
        assert!(parse_bundle_id("   ").is_err());
    }
}
