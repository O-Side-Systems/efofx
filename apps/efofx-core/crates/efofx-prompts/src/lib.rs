//! Versioned, immutable prompt registry.
//!
//! Prompts are JSON files loaded from disk at startup. Each prompt declares
//! a `name`, a semver `version`, and template strings (`system_prompt`,
//! `user_prompt_template`). Versions are immutable: once a `{name}:{version}`
//! pair is loaded, its content-hash is locked and any mismatch on reload is
//! a hard error.
//!
//! Templates support `{placeholder}` substitution. Unknown placeholders fail
//! the render. Callers pre-format numeric values — Python's `:,.0f` format
//! spec is not carried forward.
//!
//! # Example
//!
//! ```no_run
//! use efofx_prompts::PromptRegistry;
//! let registry = PromptRegistry::load_from_dir("config/prompts").unwrap();
//! let prompt = registry.latest("scoping").expect("scoping prompt loaded");
//! let user = prompt
//!     .render_user(&[
//!         ("conversation_history", "hi"),
//!         ("user_message", "my project is a pool"),
//!         ("scoping_context", "{}"),
//!     ])
//!     .unwrap();
//! # let _ = user;
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tracing::{debug, info};

/// Parsed semantic version tuple. Sort order matches semver precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemVer(pub u32, pub u32, pub u32);

impl SemVer {
    pub fn parse(s: &str) -> Result<Self, PromptError> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(PromptError::InvalidVersion(s.to_string()));
        }
        let num = |p: &str| {
            p.parse::<u32>()
                .map_err(|_| PromptError::InvalidVersion(s.to_string()))
        };
        Ok(Self(num(parts[0])?, num(parts[1])?, num(parts[2])?))
    }

    pub fn as_string(&self) -> String {
        format!("{}.{}.{}", self.0, self.1, self.2)
    }
}

/// A single loaded prompt. The `content_hash` and `source_file` fields are
/// loader-populated and not present in the JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub version: String,
    pub created_at: String,
    #[serde(default)]
    pub description: Option<String>,
    pub system_prompt: String,
    pub user_prompt_template: String,
    #[serde(skip)]
    pub content_hash: String,
    #[serde(skip)]
    pub source_file: PathBuf,
}

impl Prompt {
    /// Render `user_prompt_template` with `{key}` substitution.
    pub fn render_user(&self, vars: &[(&str, &str)]) -> Result<String, PromptError> {
        render(&self.name, &self.version, &self.user_prompt_template, vars)
    }

    /// Render `system_prompt` with `{key}` substitution. Most system prompts
    /// have no placeholders, but the API is symmetric.
    pub fn render_system(&self, vars: &[(&str, &str)]) -> Result<String, PromptError> {
        render(&self.name, &self.version, &self.system_prompt, vars)
    }
}

#[derive(Debug, Error)]
pub enum PromptError {
    #[error("prompts directory not found: {0}")]
    DirNotFound(PathBuf),
    #[error("no prompt files loaded from {0}")]
    NoPromptsLoaded(PathBuf),
    #[error("io error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid semver string: `{0}`")]
    InvalidVersion(String),
    #[error("prompt not found: {name}:{version}")]
    NotFound { name: String, version: String },
    #[error(
        "immutability violation: {name}:{version} content changed (old hash {old}, new hash {new})"
    )]
    ImmutabilityViolation {
        name: String,
        version: String,
        old: String,
        new: String,
    },
    #[error("template for {name}:{version} references `{{{placeholder}}}` which was not provided")]
    MissingPlaceholder {
        name: String,
        version: String,
        placeholder: String,
    },
    #[error("malformed placeholder in {0}: unterminated `{{`")]
    MalformedPlaceholder(String),
}

/// In-memory registry of prompts keyed by `(name, version)`.
///
/// Built once at startup with [`PromptRegistry::load_from_dir`] and shared
/// across the app via `Arc<PromptRegistry>`. All accessors are read-only.
#[derive(Debug, Default)]
pub struct PromptRegistry {
    entries: BTreeMap<(String, SemVer), Arc<Prompt>>,
}

impl PromptRegistry {
    /// Load every `*.json` file in `dir` into the registry. Fails fast on
    /// missing dir, no prompts found, invalid JSON, missing required fields
    /// (enforced by `serde`), invalid semver, or an immutability violation.
    pub fn load_from_dir(dir: impl AsRef<Path>) -> Result<Self, PromptError> {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            return Err(PromptError::DirNotFound(dir.to_path_buf()));
        }

        let mut entries: BTreeMap<(String, SemVer), Arc<Prompt>> = BTreeMap::new();
        let mut count = 0usize;

        let read_dir = fs::read_dir(dir).map_err(|e| PromptError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?;

        for entry in read_dir {
            let entry = entry.map_err(|e| PromptError::Io {
                path: dir.to_path_buf(),
                source: e,
            })?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let bytes = fs::read(&path).map_err(|e| PromptError::Io {
                path: path.clone(),
                source: e,
            })?;
            let hash = hex(Sha256::digest(&bytes).as_slice());

            let mut prompt: Prompt =
                serde_json::from_slice(&bytes).map_err(|e| PromptError::Json {
                    path: path.clone(),
                    source: e,
                })?;
            prompt.content_hash = hash;
            prompt.source_file = path.clone();

            let ver = SemVer::parse(&prompt.version)?;
            let key = (prompt.name.clone(), ver);

            if let Some(existing) = entries.get(&key) {
                if existing.content_hash != prompt.content_hash {
                    return Err(PromptError::ImmutabilityViolation {
                        name: prompt.name,
                        version: prompt.version,
                        old: existing.content_hash.clone(),
                        new: prompt.content_hash,
                    });
                }
                debug!(key = ?key, "duplicate prompt file with matching hash — skipping");
                continue;
            }

            debug!(
                name = %prompt.name,
                version = %prompt.version,
                file = %path.display(),
                "loaded prompt"
            );
            entries.insert(key, Arc::new(prompt));
            count += 1;
        }

        if count == 0 {
            return Err(PromptError::NoPromptsLoaded(dir.to_path_buf()));
        }
        info!(count, dir = %dir.display(), "prompt registry loaded");
        Ok(Self { entries })
    }

    /// Exact-version lookup. Returns `Err(PromptError::NotFound)` if absent.
    pub fn get(&self, name: &str, version: &str) -> Result<Arc<Prompt>, PromptError> {
        let ver = SemVer::parse(version)?;
        self.entries
            .get(&(name.to_string(), ver))
            .cloned()
            .ok_or_else(|| PromptError::NotFound {
                name: name.to_string(),
                version: version.to_string(),
            })
    }

    /// Highest-semver entry for `name`, or `None` if no versions are registered.
    pub fn latest(&self, name: &str) -> Option<Arc<Prompt>> {
        self.entries
            .iter()
            .filter(|((n, _), _)| n == name)
            .max_by_key(|((_, v), _)| *v)
            .map(|(_, p)| Arc::clone(p))
    }

    /// All registered versions for `name`, sorted ascending.
    pub fn versions(&self, name: &str) -> Vec<String> {
        self.entries
            .iter()
            .filter(|((n, _), _)| n == name)
            .map(|((_, v), _)| v.as_string())
            .collect()
    }

    /// All prompt names that have at least one loaded version.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.entries.keys().map(|(n, _)| n.clone()).collect();
        names.sort();
        names.dedup();
        names
    }

    /// Number of (name, version) pairs loaded.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Replace `{key}` tokens in `template` using `vars`. UTF-8 safe.
/// Unknown placeholders produce [`PromptError::MissingPlaceholder`]. There is
/// no escape syntax; literal braces are not supported.
fn render(
    name: &str,
    version: &str,
    template: &str,
    vars: &[(&str, &str)],
) -> Result<String, PromptError> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let close = after_open
            .find('}')
            .ok_or_else(|| PromptError::MalformedPlaceholder(format!("{}:{}", name, version)))?;
        let key = &after_open[..close];
        let value = vars
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
            .ok_or_else(|| PromptError::MissingPlaceholder {
                name: name.to_string(),
                version: version.to_string(),
                placeholder: key.to_string(),
            })?;
        out.push_str(value);
        rest = &after_open[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_prompt(dir: &Path, file: &str, body: &str) {
        let mut f = File::create(dir.join(file)).expect("create prompt file");
        f.write_all(body.as_bytes()).expect("write prompt body");
    }

    fn sample(name: &str, version: &str, template: &str) -> String {
        format!(
            r#"{{"name":"{name}","version":"{version}","created_at":"2026-04-22","system_prompt":"sys","user_prompt_template":"{template}"}}"#
        )
    }

    #[test]
    fn semver_parse_and_order() {
        let a = SemVer::parse("1.0.0").unwrap();
        let b = SemVer::parse("1.0.1").unwrap();
        let c = SemVer::parse("1.1.0").unwrap();
        let d = SemVer::parse("2.0.0").unwrap();
        assert!(a < b && b < c && c < d);
        assert!(SemVer::parse("1.0").is_err());
        assert!(SemVer::parse("1.0.0.0").is_err());
        assert!(SemVer::parse("1.0.x").is_err());
    }

    #[test]
    fn loads_prompts_and_computes_hash() {
        let dir = tempdir().unwrap();
        write_prompt(
            dir.path(),
            "v1.0.0-scoping.json",
            &sample("scoping", "1.0.0", "hi {user_message}"),
        );
        let reg = PromptRegistry::load_from_dir(dir.path()).unwrap();
        assert_eq!(reg.len(), 1);
        let p = reg.get("scoping", "1.0.0").unwrap();
        assert_eq!(p.system_prompt, "sys");
        assert!(!p.content_hash.is_empty());
        assert_eq!(p.content_hash.len(), 64); // SHA-256 hex
    }

    #[test]
    fn latest_picks_highest_semver() {
        let dir = tempdir().unwrap();
        write_prompt(dir.path(), "v1.0.0.json", &sample("scoping", "1.0.0", "a"));
        write_prompt(dir.path(), "v1.2.0.json", &sample("scoping", "1.2.0", "b"));
        write_prompt(dir.path(), "v2.0.1.json", &sample("scoping", "2.0.1", "c"));
        let reg = PromptRegistry::load_from_dir(dir.path()).unwrap();
        assert_eq!(reg.latest("scoping").unwrap().version, "2.0.1");
        assert_eq!(
            reg.versions("scoping"),
            vec![
                "1.0.0".to_string(),
                "1.2.0".to_string(),
                "2.0.1".to_string()
            ]
        );
    }

    #[test]
    fn ignores_non_json_files() {
        let dir = tempdir().unwrap();
        write_prompt(dir.path(), "v1.0.0.json", &sample("scoping", "1.0.0", "a"));
        write_prompt(dir.path(), "README.md", "not a prompt");
        let reg = PromptRegistry::load_from_dir(dir.path()).unwrap();
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn empty_dir_errors() {
        let dir = tempdir().unwrap();
        let err = PromptRegistry::load_from_dir(dir.path()).unwrap_err();
        assert!(matches!(err, PromptError::NoPromptsLoaded(_)));
    }

    #[test]
    fn missing_dir_errors() {
        let err = PromptRegistry::load_from_dir("/nonexistent/prompts/xyz").unwrap_err();
        assert!(matches!(err, PromptError::DirNotFound(_)));
    }

    #[test]
    fn missing_required_field_errors() {
        let dir = tempdir().unwrap();
        // Missing `user_prompt_template`
        write_prompt(
            dir.path(),
            "v1.0.0.json",
            r#"{"name":"x","version":"1.0.0","created_at":"d","system_prompt":"s"}"#,
        );
        let err = PromptRegistry::load_from_dir(dir.path()).unwrap_err();
        assert!(matches!(err, PromptError::Json { .. }));
    }

    #[test]
    fn invalid_json_errors() {
        let dir = tempdir().unwrap();
        write_prompt(dir.path(), "v1.0.0.json", "{not json");
        let err = PromptRegistry::load_from_dir(dir.path()).unwrap_err();
        assert!(matches!(err, PromptError::Json { .. }));
    }

    #[test]
    fn duplicate_identical_content_is_ok() {
        let dir = tempdir().unwrap();
        let body = sample("scoping", "1.0.0", "a");
        write_prompt(dir.path(), "v1.0.0-a.json", &body);
        write_prompt(dir.path(), "v1.0.0-b.json", &body);
        let reg = PromptRegistry::load_from_dir(dir.path()).unwrap();
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn duplicate_different_content_is_immutability_violation() {
        let dir = tempdir().unwrap();
        write_prompt(dir.path(), "a.json", &sample("scoping", "1.0.0", "a"));
        write_prompt(dir.path(), "b.json", &sample("scoping", "1.0.0", "b"));
        let err = PromptRegistry::load_from_dir(dir.path()).unwrap_err();
        assert!(matches!(err, PromptError::ImmutabilityViolation { .. }));
    }

    #[test]
    fn render_substitutes_placeholders() {
        let dir = tempdir().unwrap();
        write_prompt(
            dir.path(),
            "v1.0.0.json",
            &sample("scoping", "1.0.0", "Hi {user_message}, so far {ctx}"),
        );
        let reg = PromptRegistry::load_from_dir(dir.path()).unwrap();
        let p = reg.latest("scoping").unwrap();
        let rendered = p
            .render_user(&[("user_message", "hello"), ("ctx", "none")])
            .unwrap();
        assert_eq!(rendered, "Hi hello, so far none");
    }

    #[test]
    fn render_missing_placeholder_errors() {
        let dir = tempdir().unwrap();
        write_prompt(
            dir.path(),
            "v1.0.0.json",
            &sample("scoping", "1.0.0", "Hi {user_message}"),
        );
        let reg = PromptRegistry::load_from_dir(dir.path()).unwrap();
        let p = reg.latest("scoping").unwrap();
        let err = p.render_user(&[("other", "x")]).unwrap_err();
        match err {
            PromptError::MissingPlaceholder { placeholder, .. } => {
                assert_eq!(placeholder, "user_message");
            }
            other => panic!("wrong error: {other:?}"),
        }
    }

    #[test]
    fn render_is_utf8_safe() {
        let dir = tempdir().unwrap();
        write_prompt(
            dir.path(),
            "v1.0.0.json",
            &sample("scoping", "1.0.0", "emoji 🏗️ about {project}"),
        );
        let reg = PromptRegistry::load_from_dir(dir.path()).unwrap();
        let p = reg.latest("scoping").unwrap();
        let rendered = p.render_user(&[("project", "pool")]).unwrap();
        assert_eq!(rendered, "emoji 🏗️ about pool");
    }

    #[test]
    fn render_unterminated_placeholder_errors() {
        let dir = tempdir().unwrap();
        write_prompt(
            dir.path(),
            "v1.0.0.json",
            &sample("scoping", "1.0.0", "oops {unterminated"),
        );
        let reg = PromptRegistry::load_from_dir(dir.path()).unwrap();
        let p = reg.latest("scoping").unwrap();
        let err = p.render_user(&[("x", "y")]).unwrap_err();
        assert!(matches!(err, PromptError::MalformedPlaceholder(_)));
    }

    #[test]
    fn render_empty_vars_for_no_placeholders() {
        let dir = tempdir().unwrap();
        write_prompt(
            dir.path(),
            "v1.0.0.json",
            &sample("scoping", "1.0.0", "no placeholders here"),
        );
        let reg = PromptRegistry::load_from_dir(dir.path()).unwrap();
        let p = reg.latest("scoping").unwrap();
        assert_eq!(p.render_user(&[]).unwrap(), "no placeholders here");
    }

    #[test]
    fn names_returns_distinct_sorted() {
        let dir = tempdir().unwrap();
        write_prompt(dir.path(), "a.json", &sample("scoping", "1.0.0", "a"));
        write_prompt(dir.path(), "b.json", &sample("scoping", "2.0.0", "a"));
        write_prompt(dir.path(), "c.json", &sample("narrative", "1.0.0", "a"));
        let reg = PromptRegistry::load_from_dir(dir.path()).unwrap();
        assert_eq!(reg.names(), vec!["narrative", "scoping"]);
    }
}
