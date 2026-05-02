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
use tree_sitter::{Node, Parser};
use walkdir::WalkDir;

static SCRIPT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"m_Script:\s*\{fileID:\s*\d+,\s*guid:\s*[a-f0-9]+,\s*type:\s*3\}")
        .expect("valid SCRIPT_PATTERN regex")
});

const SUPPORTED_EXTENSIONS: [&str; 3] = [".unity", ".prefab", ".asset"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptReference {
    pub file_path: PathBuf,
    pub line_number: u32,
}

#[derive(Debug, Clone, Default)]
pub struct AnalysisResult {
    pub asset_references: Vec<ScriptReference>,
    pub class_line: Option<u32>,
}

#[derive(Debug)]
pub struct Analyzer {
    assets_dir: PathBuf,
}

impl Analyzer {
    pub fn new(workspace_root: &Uri) -> Self {
        let workspace_root = workspace_root
            .to_file_path()
            .expect("workspace_root must be a file URI");

        Self {
            assets_dir: workspace_root.join("Assets"),
        }
    }

    pub fn analyze_script(&self, content: &str, uri: Uri) -> AnalysisResult {
        let meta_path = uri.to_file_path().ok().map(|p| p.with_extension("cs.meta"));

        let asset_references = meta_path
            .and_then(|p| Self::extract_guid_from_meta(&p))
            .map(|guid| self.find_asset_references(&guid))
            .unwrap_or_default();

        AnalysisResult {
            asset_references,
            class_line: Self::get_class_line(content),
        }
    }

    fn get_class_line(content: &str) -> Option<u32> {
        let mut parser = Parser::new();
        let language = tree_sitter_c_sharp::LANGUAGE;
        parser
            .set_language(&language.into())
            .expect("Error loading CSharp parser");

        let tree = parser.parse(content, None)?;
        let root = tree.root_node();

        Self::find_class_node(root).map(|node| {
            let row = node.start_position().row;
            row as u32
        })
    }

    fn find_class_node(node: Node) -> Option<Node> {
        if node.kind() == "class_declaration" {
            return Some(node);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = Self::find_class_node(child) {
                return Some(found);
            }
        }

        None
    }

    fn extract_guid_from_meta(meta_path: &Path) -> Option<String> {
        let content = fs::read_to_string(meta_path).ok()?;
        let docs = Yaml::load_from_str(&content).ok()?;

        docs.first()?
            .as_mapping()?
            .iter()
            .find_map(|(k, v)| (k.as_str()? == "guid").then(|| v.as_str()).flatten())
            .map(str::to_owned)
    }

    fn find_asset_references(&self, script_guid: &str) -> Vec<ScriptReference> {
        let mut references = Vec::new();

        for entry in WalkDir::new(&self.assets_dir)
            .into_iter()
            .filter_map(Result::ok)
        {
            let file_path = entry.path();
            let file_name = file_path.file_name().and_then(OsStr::to_str).unwrap_or("");
            let is_asset_file = SUPPORTED_EXTENSIONS.iter().any(|x| file_name.ends_with(x));

            if !is_asset_file {
                continue;
            }

            if let Ok(content) = fs::read_to_string(entry.path()) {
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
