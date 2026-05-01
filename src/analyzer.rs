use gen_lsp_types::Uri;
use regex::Regex;
use saphyr::{LoadableYamlNode, Yaml};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};
use walkdir::WalkDir;

static SCRIPT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"m_Script:\s*\{fileID:\s*\d+,\s*guid:\s*[a-f0-9]+,\s*type:\s*3\}").unwrap()
});

const SUPPORTED_EXTENSIONS: [&str; 3] = [".unity", ".prefab", ".asset"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptReference {
    pub file_path: PathBuf,
    pub line_number: u32,
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub asset_references: Vec<ScriptReference>,
    pub class_line: Option<u32>,
}

#[derive(Debug)]
pub struct Analyzer {
    assets_folder: PathBuf,
}

impl Analyzer {
    pub fn new(workspace_root: &Uri) -> Self {
        let workspace_root = workspace_root.to_file_path().unwrap_or_default();

        Analyzer {
            assets_folder: workspace_root.join("Assets"),
        }
    }

    /// Analyze a C# script file for Unity asset references.
    /// Returns asset references where this script is used in scenes, prefabs, and assets.
    pub fn analyze_script(&self, content: &str, uri: Uri) -> AnalysisResult {
        let meta_path = uri
            .to_file_path()
            .unwrap_or_default()
            .with_added_extension("meta");

        let asset_references = Self::extract_guid_from_meta(&meta_path)
            .map(|guid| self.find_asset_references(&guid))
            .unwrap_or_default();

        AnalysisResult {
            asset_references,
            class_line: self.get_class_line(content),
        }
    }

    fn get_class_line(&self, content: &str) -> Option<u32> {
        let regex = Regex::new(r"^\s*(?:public|private|internal|protected)?\s*class\b").unwrap();

        content
            .lines()
            .enumerate()
            .find(|(_, l)| regex.is_match(l))
            .map(|(lnum, _)| lnum as u32)
    }

    /// Extract GUID from a .meta file.
    fn extract_guid_from_meta(meta_path: &Path) -> Option<String> {
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
    fn find_asset_references(&self, script_guid: &str) -> Vec<ScriptReference> {
        let mut references = Vec::new();

        for entry in WalkDir::new(&self.assets_folder)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            let file_name = file_path.file_name().and_then(OsStr::to_str).unwrap_or("");
            let is_asset_file = SUPPORTED_EXTENSIONS.iter().any(|x| file_name.ends_with(x));

            if !is_asset_file {
                continue;
            }

            if let Ok(content) = fs::read_to_string(entry.path()) {
                // Find all lines that reference this script GUID.
                for (line_number, line) in content.lines().enumerate() {
                    if SCRIPT_PATTERN.is_match(line) && line.contains(script_guid) {
                        references.push(ScriptReference {
                            file_path: file_path.to_path_buf(),
                            line_number: line_number as u32,
                        });
                    }
                }
            }
        }

        references
    }
}
