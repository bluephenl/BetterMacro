use crate::model::{MacroDocument, SCHEMA_VERSION};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn macro_dir() -> Result<PathBuf, String> {
    let mut path = dirs::data_local_dir().ok_or("Unable to find a local data directory")?;
    path.push("BetterMacro");
    path.push("macros");
    fs::create_dir_all(&path).map_err(|e| format!("Unable to create macro folder: {e}"))?;
    Ok(path)
}

fn document_path(id: &str) -> Result<PathBuf, String> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("Invalid macro id".into());
    }
    Ok(macro_dir()?.join(format!("{id}.macro.json")))
}

pub fn save(document: &MacroDocument) -> Result<(), String> {
    if document.schema_version != SCHEMA_VERSION {
        return Err("Unsupported macro schema".into());
    }
    let payload =
        serde_json::to_vec_pretty(document).map_err(|e| format!("Unable to encode macro: {e}"))?;
    let path = document_path(&document.id)?;
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = path.with_extension(format!("tmp-{}-{sequence}", std::process::id()));
    fs::write(&temporary, payload).map_err(|e| format!("Unable to write macro: {e}"))?;
    if let Err(error) = fs::rename(&temporary, &path) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Unable to finalize macro: {error}"));
    }
    Ok(())
}

pub fn load_all() -> Result<(Vec<MacroDocument>, Vec<String>), String> {
    let folder = macro_dir()?;
    let mut documents = Vec::new();
    let mut warnings = Vec::new();
    for entry in fs::read_dir(folder).map_err(|e| format!("Unable to read macro library: {e}"))? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(format!("A macro library entry could not be read: {error}."));
                continue;
            }
        };
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if !file_name.ends_with(".macro.json") {
            continue;
        }
        let raw = match fs::read(&path) {
            Ok(raw) => raw,
            Err(error) => {
                warnings.push(format!("{file_name} could not be read: {error}."));
                continue;
            }
        };
        let document: MacroDocument = match serde_json::from_slice(&raw) {
            Ok(document) => document,
            Err(error) => {
                warnings.push(format!("{file_name} is not a valid macro: {error}."));
                continue;
            }
        };
        if document.schema_version == SCHEMA_VERSION {
            documents.push(document);
        } else {
            warnings.push(format!(
                "{file_name} uses unsupported schema version {}.",
                document.schema_version
            ));
        }
    }
    documents.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok((documents, warnings))
}

pub fn delete(id: &str) -> Result<(), String> {
    let path = document_path(id)?;
    if path.exists() {
        fs::remove_file(path).map_err(|e| format!("Unable to delete macro: {e}"))?;
    }
    Ok(())
}

pub fn export(document: &MacroDocument, path: PathBuf) -> Result<(), String> {
    if document.schema_version != SCHEMA_VERSION {
        return Err("Unsupported macro schema".into());
    }
    let payload =
        serde_json::to_vec_pretty(document).map_err(|e| format!("Unable to encode macro: {e}"))?;
    fs::write(path, payload).map_err(|e| format!("Unable to export macro: {e}"))
}

pub fn import(path: PathBuf) -> Result<MacroDocument, String> {
    let raw = fs::read(path).map_err(|e| format!("Unable to read macro: {e}"))?;
    let document: MacroDocument =
        serde_json::from_slice(&raw).map_err(|e| format!("Invalid BetterMacro file: {e}"))?;
    if document.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "This macro uses schema version {}; BetterMacro currently supports version {SCHEMA_VERSION}.",
            document.schema_version
        ));
    }
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Action;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn exported_macro_can_be_imported() {
        let path = std::env::temp_dir().join(format!(
            "bettermacro-export-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let document = MacroDocument {
            schema_version: SCHEMA_VERSION,
            id: "export-test".into(),
            name: "Export test".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            modified_at: "2026-01-01T00:00:00Z".into(),
            favorite: false,
            hotkey: None,
            actions: vec![Action::Wait {
                enabled: true,
                duration_ms: 500,
            }],
        };

        export(&document, path.clone()).unwrap();
        let decoded = import(path.clone()).unwrap();
        assert_eq!(decoded.name, document.name);
        assert_eq!(decoded.actions.len(), 1);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn macro_ids_cannot_escape_or_hide_in_the_library() {
        assert!(document_path("").is_err());
        assert!(document_path("../outside").is_err());
        assert!(document_path(&"a".repeat(129)).is_err());
    }
}
