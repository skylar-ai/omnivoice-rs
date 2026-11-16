use std::process::Command;

#[cfg(not(feature = "cuda"))]
use std::{fs, path::PathBuf};

#[cfg(not(feature = "cuda"))]
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[cfg(not(feature = "cuda"))]
fn model_root() -> PathBuf {
    repo_root().join("model")
}

#[test]
fn wrapper_help_commands_exit_successfully() {
    for (binary, expected_name) in [
        (env!("CARGO_BIN_EXE_omnivoice-infer"), "omnivoice-infer"),
        (
            env!("CARGO_BIN_EXE_omnivoice-infer-batch"),
            "omnivoice-infer-batch",
        ),
    ] {
        let output = Command::new(binary).arg("--help").output().unwrap();

        assert!(
            output.status.success(),
            "stdout:\n{}\n\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("usage:"));
        assert!(stdout.contains(expected_name));
    }
}

#[test]
fn infer_subcommand_help_exits_successfully() {
    let binary = env!("CARGO_BIN_EXE_omnivoice-cli");
    let output = Command::new(binary)
        .args(["infer", "--help"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("omnivoice-cli infer"));
    assert!(stdout.contains("--guidance_scale"));
}

#[cfg(not(feature = "cuda"))]
#[test]
fn official_infer_aliases_reach_runtime_validation_without_parse_errors() {
    if !model_root().is_dir() || !repo_root().join("ref.wav").is_file() {
        return;
    }
    let binary = env!("CARGO_BIN_EXE_omnivoice-infer");
    let output_path = repo_root()
        .join("artifacts")
        .join("compat-test")
        .join("official-alias.wav");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let output = Command::new(binary)
        .args([
            "--model",
            &model_root().display().to_string(),
            "--text",
            "hello from official alias coverage",
            "--output",
            &output_path.display().to_string(),
            "--ref_audio",
            &repo_root().join("ref.wav").display().to_string(),
            "--ref_text",
            "reference transcript",
            "--num_step",
            "1",
            "--guidance_scale",
            "2.0",
            "--t_shift",
            "0.1",
            "--postprocess_output",
            "true",
            "--layer_penalty_factor",
            "5.0",
            "--position_temperature",
            "0.0",
            "--class_temperature",
            "0.0",
            "--device",
            "cuda",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("unknown argument"), "stderr:\n{stderr}");
    assert!(
        !stderr.contains("unsupported device spec"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("requires the `cuda` feature"),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        stderr
    );
}

#[test]
fn out_of_range_fade_exits_before_model_load() {
    let binary = env!("CARGO_BIN_EXE_omnivoice-cli");
    let common = [
        "--model",
        "/nonexistent/silence-controls-model",
        "--fade-duration",
        "1.5",
    ];
    let infer = [
        "infer",
        "--text",
        "hello",
        "--output",
        "/nonexistent/out.wav",
    ];
    let infer_batch = [
        "infer-batch",
        "--test-list",
        "/nonexistent/silence-controls-list.jsonl",
        "--res-dir",
        "/nonexistent/out",
    ];
    for subcommand_args in [&infer[..], &infer_batch[..]] {
        let output = Command::new(binary)
            .args(subcommand_args)
            .args(common)
            .output()
            .unwrap();

        let name = subcommand_args[0];
        assert!(!output.status.success(), "{name}: out-of-range fade must fail");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("fade_duration") || stderr.contains("fade-duration"),
            "{name}: stderr does not name the flag:\n{stderr}"
        );
        assert!(
            !stderr.contains("silence-controls-model") && !stderr.contains("silence-controls-list"),
            "{name}: the model or test list was used before validation:\n{stderr}"
        );
    }
}

#[test]
fn help_lists_silence_controls() {
    let binary = env!("CARGO_BIN_EXE_omnivoice-cli");
    for subcommand in ["infer", "infer-batch"] {
        let output = Command::new(binary)
            .args([subcommand, "--help"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{subcommand} --help failed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        for expected in [
            "--pad_duration <seconds, default 0.1>",
            "--fade_duration <seconds, default 0.1>",
            "--prompt_trailing_silence <seconds, default none>",
            "--pad-duration",
            "--fade-duration",
            "--prompt-trailing-silence",
        ] {
            assert!(
                stdout.contains(expected),
                "{subcommand} --help misses `{expected}`:\n{stdout}"
            );
        }
    }
}
