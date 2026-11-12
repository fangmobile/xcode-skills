//! Filesystem state for Xcode Coding Assistant skill installations.
//!
//! Mirrors `src/integrations.ts`, `src/filesystem-state.ts`, `src/lifecycle.ts`
//! and `src/uninstall.ts`. The filesystem is the single source of truth: no
//! manifest is kept.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationId {
    Codex,
    Claude,
}

impl IntegrationId {
    pub const ALL: [IntegrationId; 2] = [IntegrationId::Codex, IntegrationId::Claude];

    pub fn display(self) -> &'static str {
        match self {
            IntegrationId::Codex => "Codex",
            IntegrationId::Claude => "Claude",
        }
    }

    fn folder_name(self) -> &'static str {
        match self {
            IntegrationId::Codex => "codex",
            IntegrationId::Claude => "ClaudeAgentConfig",
        }
    }
}

#[derive(Clone, Debug)]
pub struct IntegrationRoot {
    pub id: IntegrationId,
    pub root: PathBuf,
    pub activated: bool,
    pub active_skills: PathBuf,
    pub disabled_skills: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillState {
    Enabled,
    Disabled,
    Conflict,
    Suspicious,
}

impl SkillState {
    pub fn label(self) -> &'static str {
        match self {
            SkillState::Enabled => "enabled",
            SkillState::Disabled => "disabled",
            SkillState::Conflict => "conflict",
            SkillState::Suspicious => "suspicious",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Skill {
    pub identity: String,
    pub state: SkillState,
    pub source: Option<String>,
    pub source_type: Option<String>,
    pub hash: Option<String>,
}

impl Skill {
    pub fn source_label(&self) -> Option<String> {
        match (&self.source, &self.source_type) {
            (Some(source), Some(kind)) => Some(format!("{kind}:{source}")),
            (Some(source), None) => Some(format!("source:{source}")),
            _ => None,
        }
    }

    pub fn short_hash(&self) -> Option<&str> {
        self.hash.as_deref().map(|hash| &hash[..hash.len().min(8)])
    }
}

/// Test/development override of the two integration roots.
#[derive(Clone, Debug, Default)]
pub struct RootOverrides {
    pub codex: Option<PathBuf>,
    pub claude: Option<PathBuf>,
}

impl RootOverrides {
    fn for_id(&self, id: IntegrationId) -> Option<&Path> {
        match id {
            IntegrationId::Codex => self.codex.as_deref(),
            IntegrationId::Claude => self.claude.as_deref(),
        }
    }
}

pub fn resolve_roots(home: &Path, overrides: &RootOverrides) -> Vec<IntegrationRoot> {
    let coding_assistant = home.join("Library/Developer/Xcode/CodingAssistant");
    IntegrationId::ALL
        .into_iter()
        .map(|id| {
            let root = overrides
                .for_id(id)
                .map(Path::to_path_buf)
                .unwrap_or_else(|| coding_assistant.join(id.folder_name()));
            IntegrationRoot {
                id,
                activated: root.exists(),
                active_skills: root.join("skills"),
                disabled_skills: root.join(".xcode-skills/disabled"),
                root,
            }
        })
        .collect()
}

pub fn scan(integration: &IntegrationRoot) -> io::Result<Vec<Skill>> {
    if !integration.activated {
        return Ok(Vec::new());
    }

    let mut identities = skill_directories(&integration.active_skills)?;
    identities.extend(skill_directories(&integration.disabled_skills)?);
    identities.sort();
    identities.dedup();

    Ok(identities
        .into_iter()
        .map(|identity| scan_skill(integration, identity))
        .collect())
}

fn skill_directories(path: &Path) -> io::Result<Vec<String>> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };

    let mut names = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || !entry.file_type()?.is_dir() {
            continue;
        }
        names.push(name);
    }
    Ok(names)
}

fn scan_skill(integration: &IntegrationRoot, identity: String) -> Skill {
    let active = integration.active_skills.join(&identity);
    let disabled = integration.disabled_skills.join(&identity);
    let active_exists = active.is_dir();
    let disabled_exists = disabled.is_dir();
    let inspected = if active_exists { &active } else { &disabled };

    let state = if !inspected.join("SKILL.md").is_file() {
        SkillState::Suspicious
    } else if active_exists && disabled_exists {
        SkillState::Conflict
    } else if active_exists {
        SkillState::Enabled
    } else {
        SkillState::Disabled
    };

    let (source, source_type, hash) = read_provenance(inspected, &identity);
    Skill {
        identity,
        state,
        source,
        source_type,
        hash,
    }
}

/// Reads `skills-lock.json` beside the skill folder; a missing or malformed
/// lock file means unknown provenance.
fn read_provenance(
    skill_path: &Path,
    identity: &str,
) -> (Option<String>, Option<String>, Option<String>) {
    let Ok(raw) = fs::read_to_string(skill_path.join("skills-lock.json")) else {
        return (None, None, None);
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return (None, None, None);
    };
    let entry = &parsed["skills"][identity];
    (
        entry["source"].as_str().map(str::to_owned),
        entry["sourceType"].as_str().map(str::to_owned),
        entry["computedHash"].as_str().map(str::to_owned),
    )
}

/// Toggles between enabled and disabled by moving the Skill Folder.
/// Returns the resulting lifecycle status.
pub fn toggle(integration: &IntegrationRoot, identity: &str) -> io::Result<&'static str> {
    if !integration.activated {
        return Ok("not-activated");
    }

    let active = integration.active_skills.join(identity);
    let disabled = integration.disabled_skills.join(identity);
    let active_exists = active.is_dir();
    let disabled_exists = disabled.is_dir();

    let (from, to, status) = match (active_exists, disabled_exists) {
        (true, true) => return Ok("conflict"),
        (true, false) => (&active, &disabled, "disabled"),
        (false, true) => (&disabled, &active, "enabled"),
        (false, false) => return Ok("not-installed"),
    };

    if !from.join("SKILL.md").is_file() {
        return Ok("suspicious");
    }

    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(from, to)?;
    Ok(status)
}

pub fn removal_targets(integration: &IntegrationRoot, identity: &str) -> Vec<PathBuf> {
    [
        integration.active_skills.join(identity),
        integration.disabled_skills.join(identity),
    ]
    .into_iter()
    .filter(|path| path.exists())
    .collect()
}

pub fn remove(paths: &[PathBuf]) -> io::Result<()> {
    for path in paths {
        match fs::remove_dir_all(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn fixture_root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "xcode-skills-gui-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn write_skill(root: &Path, kind: &str, identity: &str, lock: bool) {
        let folder = root.join(kind).join(identity);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("SKILL.md"), "# skill").unwrap();
        if lock {
            fs::write(
                folder.join("skills-lock.json"),
                format!(
                    r#"{{"skills":{{"{identity}":{{"source":"owner/repo","sourceType":"github","computedHash":"0123456789abcdef"}}}}}}"#
                ),
            )
            .unwrap();
        }
    }

    #[test]
    fn scans_and_toggles_filesystem_state() {
        let root = fixture_root();
        fs::create_dir_all(&root).unwrap();
        let overrides = RootOverrides {
            codex: Some(root.clone()),
            claude: None,
        };
        let integrations = resolve_roots(Path::new("/nonexistent-home"), &overrides);
        let codex = &integrations[0];
        assert!(codex.activated);

        write_skill(&root, "skills", "enabled-skill", true);
        write_skill(&root, ".xcode-skills/disabled", "disabled-skill", false);
        write_skill(&root, "skills", "conflict-skill", false);
        write_skill(&root, ".xcode-skills/disabled", "conflict-skill", false);
        fs::create_dir_all(root.join("skills/suspicious-skill")).unwrap();

        let skills = scan(codex).unwrap();
        let states: Vec<_> = skills
            .iter()
            .map(|skill| (skill.identity.as_str(), skill.state))
            .collect();
        assert_eq!(
            states,
            vec![
                ("conflict-skill", SkillState::Conflict),
                ("disabled-skill", SkillState::Disabled),
                ("enabled-skill", SkillState::Enabled),
                ("suspicious-skill", SkillState::Suspicious),
            ]
        );
        let enabled = &skills[2];
        assert_eq!(enabled.source.as_deref(), Some("owner/repo"));
        assert_eq!(enabled.short_hash(), Some("01234567"));

        assert_eq!(toggle(codex, "enabled-skill").unwrap(), "disabled");
        assert!(!root.join("skills/enabled-skill").exists());
        assert!(
            root.join(".xcode-skills/disabled/enabled-skill/SKILL.md")
                .is_file()
        );
        assert_eq!(toggle(codex, "enabled-skill").unwrap(), "enabled");
        assert_eq!(toggle(codex, "conflict-skill").unwrap(), "conflict");
        assert_eq!(toggle(codex, "suspicious-skill").unwrap(), "suspicious");
        assert_eq!(toggle(codex, "missing-skill").unwrap(), "not-installed");

        let targets = removal_targets(codex, "enabled-skill");
        assert_eq!(targets.len(), 1);
        remove(&targets).unwrap();
        assert_eq!(scan(codex).unwrap().len(), 3);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn unactivated_integration_has_no_skills() {
        let root = fixture_root();
        let overrides = RootOverrides {
            codex: None,
            claude: Some(root.join("missing")),
        };
        let integrations = resolve_roots(Path::new("/nonexistent-home"), &overrides);
        let claude = &integrations[1];
        assert!(!claude.activated);
        assert!(scan(claude).unwrap().is_empty());
        assert_eq!(toggle(claude, "anything").unwrap(), "not-activated");
    }
}
