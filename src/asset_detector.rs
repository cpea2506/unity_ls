use lsp_types::Uri;
use regex::Regex;
use saphyr::{LoadableYamlNode, Yaml};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use url::Url;
use walkdir::WalkDir;

static SCRIPT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"m_Script:\s*\{fileID:\s*\d+,\s*guid:\s*[a-f0-9]+,\s*type:\s*3\}").unwrap()
});

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScriptReference {
    pub file_path: PathBuf,
    pub line_number: u32,
}

pub struct AssetDetector;

impl AssetDetector {
    /// Extract GUID from a .meta file.
    pub fn extract_guid_from_meta(meta_path: &Path) -> Option<String> {
        if !meta_path.exists() {
            return None;
        }

        let content = fs::read_to_string(meta_path).ok()?;

        // YAML format: guid: <GUID>.
        let docs = Yaml::load_from_str(&content).ok()?;

        if let Some(mapping) = docs[0].as_mapping() {
            for (key, value) in mapping {
                if let Some(k) = key.as_str()
                    && k == "guid"
                    && let Some(guid) = value.as_str()
                {
                    return Some(guid.to_string());
                }
            }
        }

        None
    }

    /// Find all asset files that reference a script by GUID.
    pub fn find_asset_references(workspace_root: Uri, script_guid: &str) -> Vec<ScriptReference> {
        let mut references = Vec::new();
        let assets_path = Url::parse(workspace_root.as_str())
            .unwrap()
            .to_file_path()
            .unwrap()
            .join("Assets");

        for entry in WalkDir::new(&assets_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            let is_asset_file = file_name.ends_with(".unity")
                || file_name.ends_with(".prefab")
                || file_name.ends_with(".asset");

            if !is_asset_file {
                continue;
            }

            if let Ok(content) = fs::read_to_string(path) {
                // Find all lines that reference this script GUID.
                for (line_num, line) in content.lines().enumerate() {
                    if SCRIPT_PATTERN.is_match(line) && line.contains(script_guid) {
                        references.push(ScriptReference {
                            file_path: path.to_path_buf(),
                            line_number: line_num as u32,
                        });
                    }
                }
            }
        }

        references
    }
}
