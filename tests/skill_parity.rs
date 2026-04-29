//! Parity test for the clipmem agent-skill packages.
//!
//! Asserts that the canonical skill under
//! `skills/clipboard-memory/`, `extras/openclaw/clipboard-memory/`, and the
//! portable mirror under `extras/agent-skills/clipboard-memory/`, and the
//! Hermes-native package under `extras/hermes/clipboard-memory/` stay in sync
//! on everything that matters to a calling agent: frontmatter, documented CLI
//! surface, reference files, and the setup-check script.
//!
//! Some files are expected to be byte-identical across variants (the
//! "shared reference core"); others are allowed to diverge on metadata only
//! (SKILL.md, troubleshooting.md).

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn openclaw_pkg() -> PathBuf {
    repo_root().join("extras/openclaw/clipboard-memory")
}

fn canonical_pkg() -> PathBuf {
    repo_root().join("skills/clipboard-memory")
}

fn portable_pkg() -> PathBuf {
    repo_root().join("extras/agent-skills/clipboard-memory")
}

fn hermes_pkg() -> PathBuf {
    repo_root().join("extras/hermes/clipboard-memory")
}

/// Split a SKILL.md into (frontmatter, body).
///
/// Mirrors the loose, line-oriented parse style used by
/// `validate_openclaw_skill_content` in `src/cli/commands.rs` — we cannot pull
/// in a YAML crate because `serde_yaml` is not a dependency of this crate.
fn split_frontmatter(content: &str) -> (String, String) {
    let mut lines = content.lines();
    assert_eq!(
        lines.next(),
        Some("---"),
        "SKILL.md must begin with --- frontmatter delimiter"
    );

    let mut frontmatter = String::new();
    let mut body = String::new();
    let mut in_body = false;
    for line in lines {
        if !in_body && line == "---" {
            in_body = true;
            continue;
        }
        if in_body {
            body.push_str(line);
            body.push('\n');
        } else {
            frontmatter.push_str(line);
            frontmatter.push('\n');
        }
    }
    assert!(in_body, "SKILL.md must have a closing --- delimiter");
    (frontmatter, body)
}

/// Very small frontmatter key/value extractor.
///
/// Only handles flat `key: value` lines. Values inside a JSON blob (the
/// `metadata: {...}` line in the OpenClaw variant) are returned verbatim as a
/// single string — callers that care about JSON structure parse further.
fn get_frontmatter_value<'a>(frontmatter: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}:");
    for line in frontmatter.lines() {
        if let Some(rest) = line.strip_prefix(&prefix) {
            return Some(rest.trim());
        }
    }
    None
}

fn read_file(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

// ---------------------------------------------------------------------------
// 1. Frontmatter sanity
// ---------------------------------------------------------------------------

#[test]
fn both_skills_have_valid_frontmatter() {
    for pkg in [
        canonical_pkg(),
        openclaw_pkg(),
        portable_pkg(),
        hermes_pkg(),
    ] {
        let skill_path = pkg.join("SKILL.md");
        let content = read_file(&skill_path);
        let (frontmatter, body) = split_frontmatter(&content);

        let name = get_frontmatter_value(&frontmatter, "name")
            .unwrap_or_else(|| panic!("{} missing 'name' frontmatter", skill_path.display()));
        assert!(
            !name.is_empty(),
            "{} has empty 'name'",
            skill_path.display()
        );

        let description = get_frontmatter_value(&frontmatter, "description")
            .unwrap_or_else(|| panic!("{} missing 'description'", skill_path.display()));
        let desc_len = description.chars().count();
        assert!(
            (20..=1024).contains(&desc_len),
            "{} description length {} outside Anthropic spec range 20..=1024",
            skill_path.display(),
            desc_len
        );

        let license = get_frontmatter_value(&frontmatter, "license")
            .unwrap_or_else(|| panic!("{} missing 'license'", skill_path.display()));
        assert!(
            !license.is_empty(),
            "{} has empty 'license'",
            skill_path.display()
        );

        assert!(
            !body.trim().is_empty(),
            "{} has no body after frontmatter",
            skill_path.display()
        );
    }
}

#[test]
fn openclaw_skill_preserves_openclaw_metadata_blob() {
    // The OpenClaw runtime reads the single-line JSON under `metadata:` —
    // don't let it drift into multi-line YAML or drop required keys.
    let content = read_file(&openclaw_pkg().join("SKILL.md"));
    let (frontmatter, _) = split_frontmatter(&content);

    let metadata =
        get_frontmatter_value(&frontmatter, "metadata").expect("openclaw needs metadata");
    assert!(metadata.starts_with('{'), "openclaw metadata must be JSON");
    assert!(
        metadata.contains("\"openclaw\""),
        "openclaw metadata missing 'openclaw' key"
    );
    assert!(
        metadata.contains("\"requires\":{\"bins\":[\"clipmem\"]}"),
        "openclaw metadata missing required 'bins' gate"
    );
    assert!(
        metadata.contains("\"install\""),
        "openclaw metadata missing install descriptors"
    );
}

#[test]
fn hermes_skill_preserves_hermes_metadata() {
    let content = read_file(&hermes_pkg().join("SKILL.md"));
    let (frontmatter, _) = split_frontmatter(&content);

    let platforms =
        get_frontmatter_value(&frontmatter, "platforms").expect("hermes needs platforms");
    assert!(
        platforms.contains("macos"),
        "hermes platforms missing macos"
    );
    assert!(
        frontmatter.contains("metadata:\n  hermes:"),
        "hermes metadata missing metadata.hermes"
    );
    assert!(
        frontmatter.contains("category: productivity"),
        "hermes metadata missing productivity category"
    );
    for tag in [
        "clipboard",
        "memory",
        "macos",
        "local-first",
        "cli",
        "retrieval",
    ] {
        assert!(
            frontmatter.contains(tag),
            "hermes metadata missing tag '{tag}'"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Documented CLI surface
// ---------------------------------------------------------------------------

const CLI_COMMAND_KEYWORDS: &[&str] = &[
    // core commands
    "recall",
    "timeline",
    "search",
    "get",
    "recent",
    "export",
    "doctor",
    "agents context",
    "app settings",
    "app launch-at-login",
    "app update-check",
    "ocr candidates",
    "ocr get",
    "ocr clear",
    "storage image-candidates",
    "service providers",
    "settings reset",
    // formats
    "json",
    "toon",
    "jsonl",
    // flags / schema surface an agent must know about
    "--cursor",
    "--format",
    "schema_version",
];

const KIND_VALUES: &[&str] = &[
    "text", "html", "rtf", "url", "file", "image", "pdf", "binary", "other",
];

const EXIT_CODES: &[&str] = &["`0`", "`1`", "`2`", "`3`", "`4`", "`5`", "`6`"];

const ACTION_PARITY_KEYWORDS: &[&str] = &[
    "action parity",
    "docs/action-parity.md",
    "entity CRUD",
    "agent-accessible commands",
];

const BEHAVIOR_RULE_KEYWORDS: &[&str] = &[
    "clipmem agents context --format json",
    "recall` as a convenience ranking helper",
    "Never claim \"nothing found\"",
    "best_match_confidence",
    "quote `best_text` verbatim",
    "pbcopy",
    "open -R",
];

fn assert_contains_all(haystack: &str, needles: &[&str], where_: &Path) {
    for needle in needles {
        assert!(
            haystack.contains(needle),
            "{} is missing keyword '{}'",
            where_.display(),
            needle
        );
    }
}

#[test]
fn both_skills_document_full_cli_surface() {
    for pkg in [
        canonical_pkg(),
        openclaw_pkg(),
        portable_pkg(),
        hermes_pkg(),
    ] {
        let skill_path = pkg.join("SKILL.md");
        let commands_path = pkg.join("references/commands.md");
        let skill = read_file(&skill_path);
        let commands = read_file(&commands_path);

        // SKILL.md must name the core commands and the format rule so an
        // agent that only reads SKILL.md still knows they exist.
        assert_contains_all(&skill, CLI_COMMAND_KEYWORDS, &skill_path);
        assert_contains_all(&skill, ACTION_PARITY_KEYWORDS, &skill_path);
        assert_contains_all(&skill, BEHAVIOR_RULE_KEYWORDS, &skill_path);

        // commands.md is the deep reference — it must cover kinds and exits.
        assert_contains_all(&commands, CLI_COMMAND_KEYWORDS, &commands_path);
        assert_contains_all(&commands, KIND_VALUES, &commands_path);
        assert_contains_all(&commands, EXIT_CODES, &commands_path);
    }
}

#[test]
fn both_skills_mention_trigger_phrases() {
    // At least one concrete user-facing trigger per SKILL.md so the loader
    // has something to match on.
    let triggers = [
        "what was that command I copied?",
        "the URL I copied from Safari",
    ];
    for pkg in [openclaw_pkg(), portable_pkg(), hermes_pkg()] {
        let skill_path = pkg.join("SKILL.md");
        let skill = read_file(&skill_path);
        for trigger in triggers {
            assert!(
                skill.contains(trigger),
                "{} missing trigger phrase '{}'",
                skill_path.display(),
                trigger
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Reference file presence + byte-identical parity
// ---------------------------------------------------------------------------

const SHARED_REFERENCES: &[&str] = &[
    "references/commands.md",
    "references/json-schema.md",
    "references/examples.md",
    "references/setup-check.md",
];

const VARIANT_SPECIFIC_REFERENCES: &[&str] = &["references/troubleshooting.md"];

#[test]
fn all_variants_ship_all_reference_files() {
    for pkg in [
        canonical_pkg(),
        openclaw_pkg(),
        portable_pkg(),
        hermes_pkg(),
    ] {
        for rel in SHARED_REFERENCES
            .iter()
            .chain(VARIANT_SPECIFIC_REFERENCES.iter())
        {
            let path = pkg.join(rel);
            assert!(
                path.exists(),
                "{} is missing {}",
                pkg.display(),
                path.display()
            );
            let content = read_file(&path);
            assert!(!content.trim().is_empty(), "{} is empty", path.display());
        }
    }
}

#[test]
fn shared_references_are_byte_identical_across_variants() {
    for rel in SHARED_REFERENCES {
        let canonical = read_file(&canonical_pkg().join(rel));
        let openclaw = read_file(&openclaw_pkg().join(rel));
        let portable = read_file(&portable_pkg().join(rel));
        let hermes = read_file(&hermes_pkg().join(rel));
        assert_eq!(
            canonical, openclaw,
            "{rel} diverged between canonical and OpenClaw variants"
        );
        assert_eq!(
            canonical, portable,
            "{rel} diverged between canonical and portable variants"
        );
        assert_eq!(
            canonical, hermes,
            "{rel} diverged between canonical and Hermes variants"
        );
    }
}

#[test]
fn troubleshooting_is_non_trivial_in_both_variants() {
    // troubleshooting.md is allowed (and expected) to differ because the
    // OpenClaw variant covers sandbox/PATH issues that don't apply to the
    // portable runtime. Still, both files should be substantial.
    for pkg in [
        canonical_pkg(),
        openclaw_pkg(),
        portable_pkg(),
        hermes_pkg(),
    ] {
        let path = pkg.join("references/troubleshooting.md");
        let content = read_file(&path);
        assert!(
            content.len() > 500,
            "{} looks too small to be useful ({} bytes)",
            path.display(),
            content.len()
        );
    }
}

// ---------------------------------------------------------------------------
// 4. Setup-check script presence + exec bit
// ---------------------------------------------------------------------------

#[test]
fn setup_check_script_is_present_in_all_variants() {
    for pkg in [
        canonical_pkg(),
        openclaw_pkg(),
        portable_pkg(),
        hermes_pkg(),
    ] {
        let path = pkg.join("scripts/check-setup.sh");
        assert!(
            path.exists(),
            "{} is missing scripts/check-setup.sh",
            pkg.display()
        );
        let content = read_file(&path);
        assert!(
            content.starts_with("#!"),
            "{} missing shebang",
            path.display()
        );
        assert!(
            content.contains("clipmem"),
            "{} does not reference the clipmem binary",
            path.display()
        );
    }
}

#[test]
fn setup_check_script_is_byte_identical_across_variants() {
    let canonical_script = read_file(&canonical_pkg().join("scripts/check-setup.sh"));
    let openclaw_script = read_file(&openclaw_pkg().join("scripts/check-setup.sh"));
    let portable_script = read_file(&portable_pkg().join("scripts/check-setup.sh"));
    let hermes_script = read_file(&hermes_pkg().join("scripts/check-setup.sh"));
    assert_eq!(
        canonical_script, openclaw_script,
        "scripts/check-setup.sh diverged between canonical and OpenClaw variants"
    );
    assert_eq!(
        canonical_script, portable_script,
        "scripts/check-setup.sh diverged between canonical and portable variants"
    );
    assert_eq!(
        canonical_script, hermes_script,
        "scripts/check-setup.sh diverged between canonical and Hermes variants"
    );
}

#[cfg(unix)]
#[test]
fn setup_check_script_is_executable_in_both_variants() {
    use std::os::unix::fs::PermissionsExt;

    for pkg in [
        canonical_pkg(),
        openclaw_pkg(),
        portable_pkg(),
        hermes_pkg(),
    ] {
        let path = pkg.join("scripts/check-setup.sh");
        let perms = fs::metadata(&path)
            .unwrap_or_else(|e| panic!("cannot stat {}: {}", path.display(), e))
            .permissions();
        let mode = perms.mode() & 0o777;
        assert!(
            mode & 0o111 != 0,
            "{} is not executable (mode = {:o})",
            path.display(),
            mode
        );
    }
}
