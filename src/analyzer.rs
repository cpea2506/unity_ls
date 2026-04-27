use crate::asset_detector::{AssetDetector, ScriptReference};
use lsp_types::Uri;
use regex::Regex;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub asset_references: Vec<ScriptReference>,
    pub class_line: Option<u32>,
}

#[derive(Debug)]
pub struct UnityAnalyzer {
    workspace_root: Uri,
}

impl UnityAnalyzer {
    pub fn new(workspace_root: Uri) -> Self {
        UnityAnalyzer { workspace_root }
    }

    /// Analyze a C# script file for Unity asset references.
    /// Returns asset references where this script is used in scenes, prefabs, and assets.
    pub fn analyze_script(&self, content: &str, script_path: &Path) -> AnalysisResult {
        let mut asset_references = Vec::new();

        let meta_path = script_path.with_added_extension("meta");

        if let Some(guid) = AssetDetector::extract_guid_from_meta(&meta_path) {
            asset_references =
                AssetDetector::find_asset_references(self.workspace_root.clone(), &guid);
        }

        let class_line = self.find_class_line(content);

        AnalysisResult {
            asset_references,
            class_line,
        }
    }

    fn find_class_line(&self, content: &str) -> Option<u32> {
        let regex = Regex::new(r"^\s*(?:public|private|internal|protected)?\s*class\b").unwrap();

        for (line_num, line) in content.lines().enumerate() {
            if regex.is_match(line) {
                return Some(line_num as u32);
            }
        }

        None
    }
}
