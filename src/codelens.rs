use crate::{analyzer::AnalysisResult, asset_detector::ScriptReference};
use lsp_types::{CodeLens, Command, Location, Position, Range, Uri};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;
use url::Url;

#[derive(Serialize, Deserialize)]
struct CodeLensData {
    uri: Uri,
    asset_references: Vec<ScriptReference>,
}

pub fn create_codelens(
    analysis: AnalysisResult,
    uri: Uri,
) -> Result<Vec<CodeLens>, Box<dyn Error>> {
    let class_line = analysis.class_line.unwrap_or(0);

    let lens = CodeLens {
        range: Range {
            start: Position::new(class_line, 0),
            end: Position::new(class_line, 1),
        },
        command: None,
        data: Some(serde_json::to_value(CodeLensData {
            uri,
            asset_references: analysis.asset_references,
        })?),
    };

    Ok(vec![lens])
}

pub fn resolve_codelens(mut lens: CodeLens) -> Result<CodeLens, Box<dyn Error>> {
    let data = match lens.data.take() {
        Some(d) => d,
        None => return Ok(lens),
    };

    let CodeLensData {
        uri: _,
        asset_references,
    } = serde_json::from_value(data)?;

    let locations: Vec<Location> = asset_references
        .into_iter()
        .filter_map(|r| {
            let url = Url::from_file_path(r.file_path).ok()?;
            let uri = url.as_str().parse::<Uri>().ok()?;

            Some(Location {
                uri: uri.clone(),
                range: Range {
                    start: Position::new(r.line_number, 0),
                    end: Position::new(r.line_number, 1),
                },
            })
        })
        .collect();

    let count = locations.len();

    let title = format!(
        "{count} Unity reference{}",
        if count == 1 { "" } else { "s" }
    );

    lens.command = Some(Command {
        title,
        command: "showUnityReferences".to_string(),
        arguments: Some(vec![json!(locations)]),
    });

    Ok(lens)
}
