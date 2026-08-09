use std::{fs, path::Path, process::Command};

const SPEC_INPUT_DIR: &'static str = "tests/assets/spec";
pub const SPEC_OUTPUT_DIR: &'static str = "tests/assets/spec-generated";

/// Converts all `SPEC_INPUT_DIR`/*.wast to `SPEC_OUTPUT_DIR`/*.json.
pub fn convert_wasts() {
    let input_path = fs::read_dir(SPEC_INPUT_DIR)
        .expect(&format!("{SPEC_INPUT_DIR} should exist."));

    // delete old files
    if Path::new(SPEC_OUTPUT_DIR).exists() { 
        fs::remove_dir_all(SPEC_OUTPUT_DIR)
            .expect(&format!("Should be able to delete {SPEC_OUTPUT_DIR}."));
    }

    fs::create_dir_all(SPEC_OUTPUT_DIR)
        .expect("Should be able to create output directory.");

    let output_path = Path::new(SPEC_OUTPUT_DIR);

    // convert every .wast in the input directory to .json
    for entry in input_path {
        let path = entry.unwrap().path();

        if path.extension().map_or(false, |e| e == "wast") {
            convert_wast_to_json(&path, output_path);
        }
    }
}

/// Converts the given .wast file to its .json manifest using `wast2json`.
fn convert_wast_to_json(wast_path: &Path, output_dir: &Path) {
    let output_json = output_dir.join(
        wast_path.file_stem().unwrap()
    ).with_extension("json");

    let status = Command::new("wast2json")
        .arg(wast_path)
        .arg("-o")
        .arg(&output_json)
        .arg("--disable-saturating-float-to-int")
        .arg("--disable-sign-extension")
        .arg("--disable-simd")
        .arg("--disable-multi-value")
        .arg("--disable-bulk-memory")
        .arg("--disable-reference-types")
        .status()
        .expect("Failed to execute wast2json. Is wabt installed?");

    assert!(status.success(), "wast2json failed to convert {:?}", wast_path);
}