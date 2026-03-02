use anyhow::Context;
use std::process::Command;
use tempfile::TempDir;

fn feed_cmd() -> Command {
    Command::new("cargo")
}

#[test]
fn test_help_shows_subcommands() {
    let output = feed_cmd()
        .args(["run", "--", "--help"])
        .output()
        .expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fetch-feed"));
    assert!(stdout.contains("fetch-article"));
    assert!(stdout.contains("add"));
    assert!(stdout.contains("remove"));
    assert!(stdout.contains("list"));
    assert!(stdout.contains("tags"));
}

#[test]
fn test_list_with_empty_config() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let config_path = dir.path().join("config.yaml");
    let output = feed_cmd()
        .args([
            "run",
            "--",
            "--config",
            config_path
                .to_str()
                .context("config path is not valid UTF-8")?,
            "list",
        ])
        .output()
        .expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No feeds registered"));
    Ok(())
}

#[test]
fn test_tags_with_empty_config() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let config_path = dir.path().join("config.yaml");
    let output = feed_cmd()
        .args([
            "run",
            "--",
            "--config",
            config_path
                .to_str()
                .context("config path is not valid UTF-8")?,
            "tags",
        ])
        .output()
        .expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No tags found"));
    Ok(())
}

#[test]
fn test_help_shows_all_flag() {
    let output = feed_cmd()
        .args(["run", "--", "--help"])
        .output()
        .expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--all"));
}

#[test]
fn test_remove_nonexistent_feed() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let config_path = dir.path().join("config.yaml");
    let output = feed_cmd()
        .args([
            "run",
            "--",
            "--config",
            config_path
                .to_str()
                .context("config path is not valid UTF-8")?,
            "remove",
            "nonexistent",
        ])
        .output()
        .expect("failed to execute");
    assert!(!output.status.success());
    Ok(())
}
