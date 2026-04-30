use std::collections::{BTreeSet, HashSet};

use super::clipboard::{CaptureContext, ClipboardItem, ClipboardRepresentation, ClipboardSnapshot};
use super::kinds::{ClipboardKind, SnapshotKind};

/// Build a normalized snapshot from captured clipboard items and capture metadata.
#[must_use]
pub fn build_snapshot(capture: CaptureContext, items: Vec<ClipboardItem>) -> ClipboardSnapshot {
    ClipboardSnapshot::new_normalized(
        capture,
        fingerprint_snapshot(&items),
        snapshot_kind(&items),
        join_item_previews(&items),
        joined_search_text(&items),
        items,
    )
}

/// Build a normalized item from its captured representations.
#[must_use]
pub fn build_item(
    item_index: usize,
    representations: Vec<ClipboardRepresentation>,
) -> ClipboardItem {
    let primary = pick_primary_representation(&representations);
    let primary_kind = primary.map_or(ClipboardKind::Empty, ClipboardRepresentation::kind);
    let primary_uti = primary.map(|rep| rep.uti().to_string());

    let search_text = item_search_text(&representations);
    let preview_text = preview_for_item(&search_text, &representations);
    ClipboardItem::new(
        item_index,
        primary_kind,
        primary_uti,
        preview_text,
        search_text,
        representations,
    )
}

/// Build a normalized representation from the raw clipboard payload captured for a single UTI.
#[must_use]
pub fn build_representation(
    uti: String,
    string_value: Option<String>,
    raw_bytes: Vec<u8>,
) -> ClipboardRepresentation {
    let clean_string_value = string_value.filter(|text| is_searchable_text_fragment(text));
    let kind = classify_uti(&uti, clean_string_value.is_some());

    let decoded_text = if clean_string_value.is_some() {
        clean_string_value
    } else {
        let decoded_raw_text = decode_text_bytes_strict(&raw_bytes);
        decoded_raw_text
            .as_ref()
            .filter(|text| is_searchable_text_fragment(text))
            .map(ToString::to_string)
            .or_else(|| {
                if kind.is_textual() {
                    decode_text_bytes_lossy(&raw_bytes)
                        .filter(|text| is_searchable_text_fragment(text))
                } else {
                    decoded_raw_text
                }
            })
    };

    ClipboardRepresentation::new(uti, kind, hash_bytes(&raw_bytes), decoded_text, raw_bytes)
}

pub(crate) fn hash_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub(crate) fn classify_uti(uti: &str, text_value_present: bool) -> ClipboardKind {
    let lower = uti.to_ascii_lowercase();

    if is_origin_metadata_uti(&lower) {
        ClipboardKind::Binary
    } else if is_file_url_uti(&lower) {
        ClipboardKind::FileUrl
    } else if lower.contains("html") {
        ClipboardKind::Html
    } else if lower.contains("rtf") {
        ClipboardKind::Rtf
    } else if lower.contains("json") {
        ClipboardKind::Json
    } else if lower.contains("xml") || lower.contains("plist") {
        ClipboardKind::Xml
    } else if lower.contains("pdf") {
        ClipboardKind::Pdf
    } else if lower.contains("png")
        || lower.contains("jpeg")
        || lower.contains("jpg")
        || lower.contains("tiff")
        || lower.contains("gif")
        || lower.contains("bmp")
        || lower.contains("webp")
        || lower.contains("image")
    {
        ClipboardKind::Image
    } else if is_url_uti(&lower) {
        ClipboardKind::Url
    } else if lower.contains("text")
        || lower.contains("string")
        || lower.contains("utf8")
        || lower.contains("utf-8")
        || lower.contains("plain-text")
        || lower.contains("nsstringpboardtype")
        || text_value_present
    {
        ClipboardKind::PlainText
    } else {
        ClipboardKind::Binary
    }
}

fn is_file_url_uti(lower: &str) -> bool {
    lower.contains("file-url") || lower == "nsfilenamespboardtype"
}

fn is_url_uti(lower: &str) -> bool {
    lower == "public.url" || lower == "nsurlpboardtype"
}

fn is_origin_metadata_uti(lower: &str) -> bool {
    lower == "org.chromium.source-url" || lower == "org.chromium.source-title"
}

pub(crate) fn decode_text_bytes_strict(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return Some(String::new());
    }

    if let Some(little_endian) = detect_utf16_endianness(bytes) {
        if let Some(text) = decode_utf16_with_endian(bytes, little_endian) {
            return Some(text);
        }
    }

    if let Ok(text) = std::str::from_utf8(bytes) {
        return Some(text.to_string());
    }

    None
}

pub(crate) fn decode_text_bytes_lossy(bytes: &[u8]) -> Option<String> {
    if let Some(text) = decode_text_bytes_strict(bytes) {
        return Some(text);
    }

    if is_mostly_printable(bytes) {
        return Some(String::from_utf8_lossy(bytes).into_owned());
    }

    None
}

fn fingerprint_snapshot(items: &[ClipboardItem]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(b"clipmem/v1");

    for item in items {
        hasher.update((item.item_index() as u64).to_be_bytes());
        let mut ordered = item.representations().iter().collect::<Vec<_>>();
        ordered.sort_by(|a, b| a.uti().cmp(b.uti()));

        for rep in ordered {
            hasher.update((rep.uti().len() as u64).to_be_bytes());
            hasher.update(rep.uti().as_bytes());
            hasher.update((rep.raw_bytes().len() as u64).to_be_bytes());
            hasher.update(rep.raw_bytes());
        }
    }

    hex::encode(hasher.finalize())
}

fn snapshot_kind(items: &[ClipboardItem]) -> SnapshotKind {
    let kinds = items
        .iter()
        .map(ClipboardItem::primary_kind)
        .collect::<BTreeSet<_>>();

    if kinds.is_empty() {
        SnapshotKind::Empty
    } else if kinds.len() == 1 {
        kinds
            .into_iter()
            .next()
            .map_or(SnapshotKind::Empty, SnapshotKind::from)
    } else {
        SnapshotKind::Mixed
    }
}

pub(crate) fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let mut out = String::new();
    for (idx, ch) in text.chars().enumerate() {
        if idx >= max_chars.saturating_sub(1) {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

pub(crate) fn normalize_whitespace(text: &str) -> String {
    let mut parts = text.split_whitespace();
    let Some(first) = parts.next() else {
        return String::new();
    };

    let mut out = String::with_capacity(text.len());
    out.push_str(first);
    for part in parts {
        out.push(' ');
        out.push_str(part);
    }
    out
}

pub(crate) fn dedupe_text_fragments<I>(fragments: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for fragment in fragments {
        let collapsed = normalize_whitespace(&fragment);
        let trimmed = collapsed.trim();
        if trimmed.is_empty() {
            continue;
        }

        let key = trimmed.to_lowercase();
        if seen.insert(key) {
            out.push(trimmed.to_string());
        }
    }

    out
}

pub(crate) fn is_searchable_text_fragment(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.contains('\0') {
        return false;
    }

    let mut total = 0;
    let mut disruptive_controls = 0;
    for ch in trimmed.chars() {
        total += 1;
        if ch.is_control() && !matches!(ch, '\n' | '\r' | '\t') {
            disruptive_controls += 1;
        }
    }

    ratio_at_most(disruptive_controls, total, 1, 20)
}

fn detect_utf16_endianness(bytes: &[u8]) -> Option<bool> {
    if bytes.len() < 2 || !bytes.len().is_multiple_of(2) {
        return None;
    }

    if bytes.starts_with(&[0xff, 0xfe]) {
        return Some(true);
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        return Some(false);
    }

    let lane_len = bytes.len() / 2;
    let even_nul_count = nul_count_for_lane(bytes, 0);
    let odd_nul_count = nul_count_for_lane(bytes, 1);

    if ratio_at_least(even_nul_count, lane_len, 6, 10)
        && ratio_at_most(odd_nul_count, lane_len, 2, 10)
    {
        Some(false)
    } else if ratio_at_least(odd_nul_count, lane_len, 6, 10)
        && ratio_at_most(even_nul_count, lane_len, 2, 10)
    {
        Some(true)
    } else {
        None
    }
}

fn nul_count_for_lane(bytes: &[u8], lane: usize) -> usize {
    bytes
        .iter()
        .skip(lane)
        .step_by(2)
        .filter(|&&byte| byte == 0)
        .count()
}

fn ratio_at_least(count: usize, total: usize, numerator: usize, denominator: usize) -> bool {
    count.saturating_mul(denominator) >= total.saturating_mul(numerator)
}

fn ratio_at_most(count: usize, total: usize, numerator: usize, denominator: usize) -> bool {
    count.saturating_mul(denominator) <= total.saturating_mul(numerator)
}

fn decode_utf16_with_endian(bytes: &[u8], little_endian: bool) -> Option<String> {
    let mut words = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let word = if little_endian {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        };
        words.push(word);
    }

    if matches!(words.first().copied(), Some(0xfeff | 0xfffe)) {
        words.remove(0);
    }

    String::from_utf16(&words).ok()
}

fn is_mostly_printable(bytes: &[u8]) -> bool {
    let printable = bytes
        .iter()
        .filter(|&&byte| {
            byte == b'\n' || byte == b'\r' || byte == b'\t' || (0x20..=0x7e).contains(&byte)
        })
        .count();

    ratio_at_least(printable, bytes.len(), 85, 100)
}

pub(crate) fn html_to_text_lossy(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut entity = String::new();
    let mut in_entity = false;

    for ch in html.chars() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
                out.push(' ');
            }
            continue;
        }

        if in_entity {
            if ch == ';' {
                out.push_str(&decode_html_entity(&entity).unwrap_or_else(|| " ".to_string()));
                entity.clear();
                in_entity = false;
                continue;
            }
            if entity.len() < 12 && is_html_entity_char(ch) {
                entity.push(ch);
                continue;
            }

            out.push('&');
            out.push_str(&entity);
            entity.clear();
            in_entity = false;
        }

        match ch {
            '<' => in_tag = true,
            '&' => {
                in_entity = true;
                entity.clear();
            }
            _ => out.push(ch),
        }
    }

    if in_entity {
        out.push('&');
        out.push_str(&entity);
    }

    normalize_whitespace(&out)
}

fn is_html_entity_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '#'
}

fn decode_html_entity(entity: &str) -> Option<String> {
    match entity {
        "amp" => Some("&".to_string()),
        "lt" => Some("<".to_string()),
        "gt" => Some(">".to_string()),
        "quot" => Some("\"".to_string()),
        "apos" => Some("'".to_string()),
        "nbsp" => Some(" ".to_string()),
        _ => decode_numeric_html_entity(entity).map(|ch| ch.to_string()),
    }
}

fn decode_numeric_html_entity(entity: &str) -> Option<char> {
    if let Some(hex) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
    } else if let Some(decimal) = entity.strip_prefix('#') {
        decimal.parse::<u32>().ok().and_then(char::from_u32)
    } else {
        None
    }
}

pub(crate) fn rtf_to_text_lossy(rtf: &str) -> String {
    let mut out = String::with_capacity(rtf.len());
    let mut chars = rtf.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        match ch {
            '{' | '}' => {
                continue;
            }
            '\\' => {
                let Some((escaped_index, escaped)) = chars.next() else {
                    break;
                };

                match escaped {
                    '\\' | '{' | '}' => {
                        out.push(escaped);
                    }
                    '\'' => {
                        let high = chars.next().and_then(|(_, hex)| hex.to_digit(16));
                        let low = chars.next().and_then(|(_, hex)| hex.to_digit(16));
                        if let (Some(high), Some(low)) = (high, low) {
                            out.push(((high << 4) as u8 | low as u8) as char);
                        }
                    }
                    c if c.is_ascii_alphabetic() => {
                        let word_start = escaped_index;
                        let mut word_end = escaped_index + escaped.len_utf8();
                        while let Some(&(byte_index, next)) = chars.peek() {
                            if !next.is_ascii_alphabetic() {
                                break;
                            }
                            word_end = byte_index + next.len_utf8();
                            chars.next();
                        }
                        let word = &rtf[word_start..word_end];

                        if let Some(&(_, next)) = chars.peek() {
                            if next == '-' || next.is_ascii_digit() {
                                chars.next();
                                while let Some(&(_, digit)) = chars.peek() {
                                    if !digit.is_ascii_digit() {
                                        break;
                                    }
                                    chars.next();
                                }
                            }
                        }

                        if let Some(&(_, ' ')) = chars.peek() {
                            chars.next();
                        }

                        match word {
                            "par" | "line" => out.push('\n'),
                            "tab" => out.push('\t'),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            ch => {
                out.push(ch);
            }
        }
    }

    normalize_whitespace(&out)
}

#[cfg(test)]
#[path = "builders/profile_tests.rs"]
mod profile_tests;

fn pick_primary_representation(
    reps: &[ClipboardRepresentation],
) -> Option<&ClipboardRepresentation> {
    reps.iter()
        .min_by_key(|rep| (rep.kind().priority(), !rep.is_text(), rep.uti()))
}

fn item_search_text(representations: &[ClipboardRepresentation]) -> String {
    let fragments = representations
        .iter()
        .filter_map(search_fragment_for_representation);
    dedupe_text_fragments(fragments).join("\n\n")
}

fn search_fragment_for_representation(rep: &ClipboardRepresentation) -> Option<String> {
    let text = rep.text_value()?;

    match rep.kind() {
        ClipboardKind::Html => Some(html_to_text_lossy(text)),
        ClipboardKind::Rtf => Some(rtf_to_text_lossy(text)),
        kind if kind.is_textual() => Some(text.to_string()),
        _ => None,
    }
}

fn preview_for_item(search_text: &str, representations: &[ClipboardRepresentation]) -> String {
    if !search_text.is_empty() {
        return truncate_chars(&search_text.replace('\n', " "), 200);
    }

    if let Some(rep) = pick_primary_representation(representations) {
        return truncate_chars(
            &format!(
                "[{} · {} bytes · {}]",
                rep.kind(),
                rep.byte_len(),
                rep.uti()
            ),
            200,
        );
    }

    "[empty clipboard item]".to_string()
}

fn join_item_previews(items: &[ClipboardItem]) -> String {
    let parts = items
        .iter()
        .map(|item| item.preview_text().to_string())
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>();

    if parts.is_empty() {
        "[empty clipboard]".to_string()
    } else {
        truncate_chars(&parts.join(" | "), 280)
    }
}

fn joined_search_text(items: &[ClipboardItem]) -> String {
    items
        .iter()
        .map(|item| item.search_text().to_string())
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::super::clipboard::CaptureContext;
    use super::super::kinds::{ClipboardKind, SnapshotKind};
    use super::{
        build_item, build_representation, build_snapshot, decode_text_bytes_lossy,
        html_to_text_lossy, rtf_to_text_lossy,
    };

    #[test]
    fn html_is_stripped_reasonably() {
        let html = "<p>Hello <strong>world</strong> &amp; friends</p>";
        assert_eq!(html_to_text_lossy(html), "Hello world & friends");
    }

    #[test]
    fn html_decodes_numeric_entities() {
        let html = "<p>Tristan&#39;s path: &#x2F;tmp&#47;clipmem</p>";
        assert_eq!(html_to_text_lossy(html), "Tristan's path: /tmp/clipmem");
    }

    #[test]
    fn html_preserves_bare_ampersands() {
        let html = "<p>AT&T research</p>";
        assert_eq!(html_to_text_lossy(html), "AT&T research");
    }

    #[test]
    fn rtf_is_stripped_reasonably() {
        let rtf = r"{\rtf1\ansi hello\par world}";
        assert_eq!(rtf_to_text_lossy(rtf), "hello world");
    }

    #[test]
    fn utf16_decoding_works() {
        let bytes = "hello"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(decode_text_bytes_lossy(&bytes).as_deref(), Some("hello"));
    }

    #[test]
    fn embedded_nul_string_values_fall_back_to_decoded_raw_bytes() {
        let raw_bytes = "clipmem apple text"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let string_value = Some(
            "clipmem apple text"
                .chars()
                .flat_map(|ch| [ch, '\0'])
                .collect::<String>(),
        );

        let representation = build_representation(
            "public.utf16-plain-text".to_string(),
            string_value,
            raw_bytes,
        );

        assert_eq!(representation.text_value(), Some("clipmem apple text"));
    }

    #[test]
    fn apple_plain_text_capture_ignores_control_only_dynamic_representation() {
        let utf16_bytes = "clipmem-e2e-text"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let utf16_nul_text = "clipmem-e2e-text"
            .chars()
            .flat_map(|ch| [ch, '\0'])
            .collect::<String>();
        let item = build_item(
            0,
            vec![
                build_representation(
                    "public.utf16-plain-text".to_string(),
                    Some(utf16_nul_text),
                    utf16_bytes,
                ),
                build_representation(
                    "public.utf8-plain-text".to_string(),
                    Some("clipmem-e2e-text".to_string()),
                    b"clipmem-e2e-text".to_vec(),
                ),
                build_representation(
                    "com.apple.traditional-mac-plain-text".to_string(),
                    Some("clipmem-e2e-text".to_string()),
                    b"clipmem-e2e-text".to_vec(),
                ),
                build_representation(
                    "dyn.ah62d4rv4gk81g7d3ru".to_string(),
                    Some("\u{1}\0\0\0\0\0\u{10}\0".to_string()),
                    vec![1, 0, 0, 0, 0, 0, 16, 0],
                ),
            ],
        );
        let snapshot = build_snapshot(CaptureContext::new(4), vec![item]);

        assert_eq!(snapshot.search_text(), "clipmem-e2e-text");
        assert_eq!(snapshot.preview_text(), "clipmem-e2e-text");
    }

    #[test]
    fn utf16le_bom_decoding_works() {
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend("hello".encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(decode_text_bytes_lossy(&bytes).as_deref(), Some("hello"));
    }

    #[test]
    fn utf16be_decoding_works() {
        let bytes = "hello"
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect::<Vec<_>>();
        assert_eq!(decode_text_bytes_lossy(&bytes).as_deref(), Some("hello"));
    }

    #[test]
    fn empty_snapshots_stay_structurally_empty() {
        let snapshot = build_snapshot(CaptureContext::new(7), Vec::new());

        assert_eq!(snapshot.snapshot_kind(), SnapshotKind::Empty);
        assert_eq!(snapshot.item_count(), 0);
        assert!(snapshot.items().is_empty());
        assert_eq!(snapshot.preview_text(), "[empty clipboard]");
    }

    #[test]
    fn generic_utf8_payloads_decode_without_becoming_searchable() {
        let representation =
            build_representation("public.data".to_string(), None, b"hello".to_vec());
        let item = build_item(0, vec![representation.clone()]);

        assert_eq!(representation.kind(), ClipboardKind::Binary);
        assert_eq!(representation.text_value(), Some("hello"));
        assert!(item.search_text().is_empty());
    }

    #[test]
    fn chromium_source_url_metadata_is_not_copied_url_content() {
        let item = build_item(
            0,
            vec![
                build_representation(
                    "org.chromium.source-url".to_string(),
                    Some("https://example.com/source-page".to_string()),
                    b"https://example.com/source-page".to_vec(),
                ),
                build_representation(
                    "public.utf8-plain-text".to_string(),
                    Some("selected article text".to_string()),
                    b"selected article text".to_vec(),
                ),
            ],
        );

        assert_eq!(item.primary_kind(), ClipboardKind::PlainText);
        assert_eq!(item.search_text(), "selected article text");
    }

    #[test]
    fn generic_utf16_payloads_decode_without_becoming_searchable() {
        let bytes = "hello"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let representation = build_representation("public.data".to_string(), None, bytes);
        let item = build_item(0, vec![representation.clone()]);

        assert_eq!(representation.kind(), ClipboardKind::Binary);
        assert_eq!(representation.text_value(), Some("hello"));
        assert!(item.search_text().is_empty());
    }

    #[test]
    fn binary_payloads_still_do_not_decode_lossily() {
        let representation = build_representation(
            "public.data".to_string(),
            None,
            vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
        );

        assert_eq!(representation.kind(), ClipboardKind::Binary);
        assert_eq!(representation.text_value(), None);
    }
}
