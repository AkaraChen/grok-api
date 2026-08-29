use serde::Serialize;

use crate::error::Result;
use crate::model::config::OutputFormat;

pub fn emit(format: OutputFormat, quiet: bool, text: impl AsRef<str>, json: &impl Serialize) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(json)?);
        }
        OutputFormat::Text if quiet => {}
        OutputFormat::Text => {
            let text = text.as_ref();
            if !text.is_empty() {
                println!("{text}");
            }
        }
    }
    Ok(())
}

pub fn emit_text(format: OutputFormat, quiet: bool, text: impl AsRef<str>) -> Result<()> {
    if matches!(format, OutputFormat::Json) {
        emit(format, quiet, "", &serde_json::json!({ "message": text.as_ref() }))
    } else {
        emit(format, quiet, text, &serde_json::json!({}))
    }
}
