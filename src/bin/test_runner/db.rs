use lava_forecaster::models::UnifiedTestCase;
use std::fs;
use std::path::Path;

pub const LTP_MAGIC: &[u8; 4] = b"LTP\x01";

pub fn read_test_pack(path: &Path) -> Result<Vec<UnifiedTestCase>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes =
        fs::read(path).map_err(|e| format!("Failed to read test pack {:?}: {}", path, e))?;
    if bytes.len() < 4 || &bytes[0..4] != LTP_MAGIC {
        return Err(format!(
            "Invalid test pack format: missing LTP magic header"
        ));
    }
    rmp_serde::from_slice(&bytes[4..])
        .map_err(|e| format!("Failed to deserialize test pack: {}", e))
}

pub fn write_test_pack(path: &Path, cases: &[UnifiedTestCase]) -> Result<(), String> {
    let mut bytes = LTP_MAGIC.to_vec();
    let mut buf = Vec::new();
    let mut serializer = rmp_serde::Serializer::new(&mut buf).with_struct_map();
    serde::Serialize::serialize(cases, &mut serializer)
        .map_err(|e| format!("Failed to serialize test pack: {}", e))?;
    bytes.extend_from_slice(&buf);
    fs::write(path, bytes).map_err(|e| format!("Failed to write test pack {:?}: {}", path, e))
}

pub fn load_cases_from_source(source: &Path) -> Result<Vec<UnifiedTestCase>, String> {
    if source.is_dir() {
        let entries = fs::read_dir(source)
            .map_err(|e| format!("Failed to read directory {:?}: {}", source, e))?;
        let mut test_cases = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            let extension = path.extension().map_or("", |e| e.to_str().unwrap_or(""));
            if extension == "json" || extension == "test" {
                let content = fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;
                let tc_opt = if extension == "json" {
                    serde_json::from_str::<UnifiedTestCase>(&content).ok()
                } else {
                    lava_forecaster::test_dsl::parse_test_case_dsl(&content).ok()
                };
                if let Some(tc) = tc_opt {
                    test_cases.push(tc);
                }
            }
        }
        Ok(test_cases)
    } else if source.is_file() {
        let extension = source.extension().map_or("", |e| e.to_str().unwrap_or(""));
        if extension == "json" || extension == "test" {
            let content = fs::read_to_string(source)
                .map_err(|e| format!("Failed to read file {:?}: {}", source, e))?;
            let tc = if extension == "json" {
                serde_json::from_str::<UnifiedTestCase>(&content)
                    .map_err(|e| format!("Failed to parse JSON: {}", e))?
            } else {
                lava_forecaster::test_dsl::parse_test_case_dsl(&content)
                    .map_err(|e| format!("Failed to parse DSL: {}", e))?
            };
            Ok(vec![tc])
        } else {
            read_test_pack(source)
        }
    } else {
        Err(format!(
            "Source path {:?} does not exist or is not a file/directory",
            source
        ))
    }
}

pub fn cleanup_empty_dirs(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        let mut is_empty = true;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                cleanup_empty_dirs(&path);
            }
            if path.exists() {
                is_empty = false;
            }
        }
        if is_empty && dir.to_str().map_or(false, |s| s.contains("cases")) {
            let _ = fs::remove_dir(dir);
        }
    }
}
