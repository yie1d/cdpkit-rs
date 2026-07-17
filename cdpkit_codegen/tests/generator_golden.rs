use cdpkit_codegen::{generator, parser::Protocol};

fn generated_output() -> String {
    let protocol: Protocol =
        serde_json::from_str(include_str!("fixtures/mini_protocol.json")).unwrap();
    generator::generate_code(&[protocol])
}

fn assert_contains_golden(output: &str, golden: &str) {
    let expected = std::fs::read_to_string(format!(
        "{}/tests/goldens/{golden}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let normalize = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized_output = normalize(output);
    let normalized_expected = normalize(&expected);

    assert!(
        normalized_output.contains(&normalized_expected),
        "generated output did not contain golden snippet {golden}\n--- expected snippet ---\n{expected}\n--- generated output ---\n{output}"
    );
}

#[test]
fn generate_code_is_stable_for_fixed_fixture() {
    let first = generated_output();
    let second = generated_output();

    assert_eq!(first, second, "generator output should be byte-stable");
}

#[test]
fn golden_covers_command_builders_and_flatten_validation() {
    let output = generated_output();

    assert_contains_golden(&output, "command_builder.rs");
    assert_contains_golden(&output, "set_auto_attach.rs");
}

#[test]
fn golden_covers_event_subscription_variants() {
    let output = generated_output();

    assert_contains_golden(&output, "event_subscription.rs");
}

#[test]
fn golden_covers_enum_refs_and_keyword_renames() {
    let output = generated_output();

    assert_contains_golden(&output, "enum_mode.rs");
    assert_contains_golden(&output, "keyword_carrier.rs");
    assert_contains_golden(&output, "inspect_command.rs");
}
