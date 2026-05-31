use anyhow::Result;
use std::collections::BTreeSet;
use std::fs;
use tempfile::tempdir;

use apogee::{Config, ContextEnv, RuntimeEnv, Shell};

#[test]
fn host_env_derived_can_reference_base_env_derived_in_emit_order() -> Result<()> {
    let dir = tempdir()?;
    let cfg_path = dir.path().join("config.toml");

    fs::write(
        &cfg_path,
        r#"
[apogee]
schema_version = 2
default_shell = "zsh"
env_file = "/tmp/apogee-test-missing.env"

[modules]
enable_cloud = true
enable_apps = false
enable_hooks = false
enable_templates = false

[modules.cloud]
enabled = true

[modules.cloud.example]
enabled = true
kind = "storage"
platforms = ["mac"]

[modules.cloud.example.detect.paths.mac]
any_of = ["{home}"]

[modules.cloud.example.emit.env]
ROOT = "/base"

[modules.cloud.example.emit.env_derived]
A = "$ROOT/a"

[modules.cloud.example.emit.env_derived_hosts.quasar]
B = "$A/b"
"#,
    )?;

    let mut ctx = ContextEnv::new()?;
    ctx.host = "quasar".to_string();

    let cfg = Config::load_from_path(&cfg_path)?;
    let rt0 = RuntimeEnv::build(&ctx, &cfg)?;

    let mut work = rt0.clone();
    let mut active = BTreeSet::new();

    let out = apogee::emit_cloud_seq(&ctx, &mut work, &cfg, Shell::Zsh, &mut active)?;

    assert!(out.contains(r#"export ROOT="/base""#));
    assert!(out.contains(r#"export A="$ROOT/a""#));
    assert!(out.contains(r#"export B="$A/b""#));

    let pos_root = out.find(r#"export ROOT="/base""#).unwrap();
    let pos_a = out.find(r#"export A="$ROOT/a""#).unwrap();
    let pos_b = out.find(r#"export B="$A/b""#).unwrap();

    assert!(pos_root < pos_a);
    assert!(pos_a < pos_b);

    assert_eq!(work.vars.get("ROOT").map(String::as_str), Some("/base"));
    assert_eq!(work.vars.get("A").map(String::as_str), Some("$ROOT/a"));
    assert_eq!(work.vars.get("B").map(String::as_str), Some("$A/b"));

    Ok(())
}
