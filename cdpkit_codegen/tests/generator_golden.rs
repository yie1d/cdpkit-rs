use cdpkit_codegen::{generator, parser::Protocol};
use std::path::PathBuf;

fn generated_output() -> String {
    let protocol: Protocol =
        serde_json::from_str(include_str!("fixtures/mini_protocol.json")).unwrap();
    generator::generate_code(&[protocol])
}

fn complete_golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/mini_protocol.rs")
}

fn complete_golden() -> String {
    let mut expected = std::fs::read_to_string(complete_golden_path()).unwrap();
    expected.push('\n');
    expected
}

fn assert_complete_golden(output: &str) {
    let expected = complete_golden();
    assert_eq!(
        output, expected,
        "generated output differed from the complete mini protocol golden"
    );
}

#[test]
fn generated_output_matches_complete_golden() {
    assert_complete_golden(&generated_output());
}

#[test]
fn complete_golden_rejects_duplicated_output() {
    let output = generated_output();
    let duplicated = format!("{output}{output}");

    assert_ne!(duplicated, complete_golden());
}

#[test]
fn generated_header_is_static_without_wall_clock_metadata() {
    let output = generated_output();
    let header = output.lines().take(2).collect::<Vec<_>>();

    assert_eq!(
        header,
        [
            "// Auto-generated from Chrome DevTools Protocol",
            "// DO NOT EDIT MANUALLY  OvO",
        ]
    );
}

#[test]
#[ignore = "maintenance helper: run explicitly after intentional generator changes"]
fn update_complete_golden() {
    let output = generated_output();
    let file_contents = output
        .strip_suffix('\n')
        .expect("generated output must end with a newline");
    std::fs::write(complete_golden_path(), file_contents).unwrap();
}
