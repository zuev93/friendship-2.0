use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("build");

    match cmd {
        "build" => {
            if !run("cargo", &["build"]) {
                return ExitCode::FAILURE;
            }
            clean_generated();
            if !run_tests() {
                return ExitCode::FAILURE;
            }
            eprintln!("\n=== Build + tests passed ===");
            ExitCode::SUCCESS
        }
        "test" => {
            clean_generated();
            if !run_tests() {
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("Usage: cargo xtask [build|test]");
            eprintln!("  build  - firmware build + all tests (default)");
            eprintln!("  test   - screenshot + DSP tests only");
            ExitCode::FAILURE
        }
    }
}

fn host_target() -> &'static str {
    if cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else if cfg!(target_os = "macos") {
        "aarch64-apple-darwin"
    } else {
        "x86_64-unknown-linux-gnu"
    }
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn clean_generated() {
    let docs = project_root().join("docs");
    if let Ok(entries) = std::fs::read_dir(&docs) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("screen-") && name.ends_with(".png") {
                    let _ = std::fs::remove_file(&path);
                    eprintln!("Cleaned: {}", path.display());
                }
            }
        }
    }
    let test_output = project_root().join("main-controller/test-output");
    if test_output.exists() {
        let _ = std::fs::remove_dir_all(&test_output);
        eprintln!("Cleaned: {}", test_output.display());
    }
}

fn run_tests() -> bool {
    if !run(
        "cargo",
        &[
            "test",
            "--target",
            host_target(),
            "-p",
            "druzhba-front-panel-controller",
            "--no-default-features",
            "--test",
            "screenshots",
        ],
    ) {
        return false;
    }
    if !run(
        "cargo",
        &[
            "test",
            "--target",
            host_target(),
            "-p",
            "druzhba-main-controller",
            "--no-default-features",
            "--test",
            "dsp_loopback",
            "--",
            "--nocapture",
        ],
    ) {
        return false;
    }
    run(
        "cargo",
        &[
            "test",
            "--target",
            host_target(),
            "-p",
            "druzhba-main-controller",
            "--no-default-features",
            "--test",
            "dsp_features",
            "--",
            "--nocapture",
        ],
    )
}

fn run(program: &str, args: &[&str]) -> bool {
    eprintln!("\n> {} {}", program, args.join(" "));
    Command::new(program)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
