use crate::analyzer::{AnalysisResult, ScriptReference};
use gen_lsp_types::{CodeLens, Command, Location, Position, Range, Uri};
use serde_json::json;
use std::{error::Error, str::FromStr};

pub fn create_codelens(analysis: AnalysisResult) -> Result<Vec<CodeLens>, Box<dyn Error>> {
    let class_line = analysis.class_line.unwrap_or(0);

    let lens = CodeLens {
        range: Range {
            start: Position::new(class_line, 0),
            end: Position::new(class_line, 1),
        },
        command: None,
        data: Some(serde_json::to_value(analysis.asset_references)?),
    };

    Ok(vec![lens])
}

pub fn resolve_codelens(mut lens: CodeLens) -> Result<CodeLens, Box<dyn Error>> {
    let data = match lens.data.take() {
        Some(d) => d,
        None => return Ok(lens),
    };

    let asset_references = serde_json::from_value::<Vec<ScriptReference>>(data)?;

    let locations = asset_references
        .into_iter()
        .filter_map(|r| {
            let uri = Uri::from_str(r.file_path.to_str()?).ok()?;

            Some(Location {
                uri: uri.clone(),
                range: Range {
                    start: Position::new(r.line_number, 0),
                    end: Position::new(r.line_number, 1),
                },
            })
        })
        .collect::<Vec<Location>>();

    let count = locations.len();

    let title = format!(
        "{count} Unity reference{}",
        if count == 1 { "" } else { "s" }
    );

    lens.command = Some(Command {
        title,
        command: "showUnityReferences".to_string(),
        arguments: Some(vec![json!(locations)]),
        ..Default::default()
    });

    Ok(lens)
}
