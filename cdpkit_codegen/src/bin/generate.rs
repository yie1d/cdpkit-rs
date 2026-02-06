use cdpkit_codegen::{fetch, generator, parser};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let update = args.contains(&"--update".to_string());

    // Protocol files in cdpkit_codegen/protocol
    let protocol_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("protocol");

    if update {
        println!("Downloading latest protocol files to {:?}...", protocol_dir);
        std::fs::create_dir_all(&protocol_dir)?;
        fetch::fetch_protocols(&protocol_dir).await?;
    }

    println!("Parsing protocol files from {:?}...", protocol_dir);
    let browser_json = std::fs::read_to_string(protocol_dir.join("browser_protocol.json"))?;
    let js_json = std::fs::read_to_string(protocol_dir.join("js_protocol.json"))?;

    let browser_protocol: parser::Protocol = serde_json::from_str(&browser_json)?;
    let js_protocol: parser::Protocol = serde_json::from_str(&js_json)?;

    println!("Generating code...");
    let code = generator::generate_code(&[browser_protocol, js_protocol]);

    // Output to cdpkit/src/cdp.rs (relative to workspace root)
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let output_path = workspace_root.join("cdpkit/src/cdp.rs");
    std::fs::write(&output_path, code)?;

    println!("Code generated successfully at {:?}", output_path);

    println!("Running cargo fmt...");
    let status = std::process::Command::new("cargo")
        .arg("fmt")
        .arg("--manifest-path")
        .arg(workspace_root.join("cdpkit/Cargo.toml"))
        .status()?;

    if status.success() {
        println!("Code formatted successfully.");
    } else {
        eprintln!("Warning: cargo fmt failed with status: {}", status);
    }

    Ok(())
}
