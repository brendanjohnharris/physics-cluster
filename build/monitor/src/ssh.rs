use std::fs;
use std::io::Write;
use std::path::Path;

const MARKER: &str = "# monitor: headnode connection multiplexing";

pub fn ensure_controlmaster_in(config: &Path, control_dir: &Path) -> std::io::Result<bool> {
    fs::create_dir_all(control_dir)?;

    let existing = fs::read_to_string(config).unwrap_or_default();
    if existing.contains(MARKER) {
        return Ok(false);
    }

    let cdir = control_dir.display();
    let block = format!(
        "\n{MARKER}\nHost headnode\n    ControlMaster auto\n    ControlPath {cdir}/cm-%r@%h:%p\n    ControlPersist 60s\n"
    );

    if let Some(parent) = config.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(config)?;
    f.write_all(block.as_bytes())?;
    Ok(true)
}

pub fn ensure_controlmaster() -> std::io::Result<bool> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let ssh_dir = Path::new(&home).join(".ssh");
    let config = ssh_dir.join("config");
    let control_dir = ssh_dir.join("cm");
    ensure_controlmaster_in(&config, &control_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn tmp(tag: &str) -> PathBuf {
        let mut d = std::env::temp_dir();
        d.push(format!("monitor_sshtest_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn adds_block_once_and_is_idempotent() {
        let dir = tmp("cm");
        let cfg = dir.join("config");
        fs::write(&cfg, "Host other\n    User x\n").unwrap();
        let cdir = dir.join("cm");

        let changed = ensure_controlmaster_in(&cfg, &cdir).unwrap();
        assert!(changed);
        let body = fs::read_to_string(&cfg).unwrap();
        assert!(body.contains(MARKER));
        assert!(body.contains("Host headnode"));
        assert!(body.contains("ControlMaster auto"));
        assert!(cdir.is_dir());

        // Second call must not duplicate.
        let changed2 = ensure_controlmaster_in(&cfg, &cdir).unwrap();
        assert!(!changed2);
        let body2 = fs::read_to_string(&cfg).unwrap();
        assert_eq!(body2.matches(MARKER).count(), 1);
    }

    #[test]
    fn creates_config_when_missing() {
        let dir = tmp("new");
        let cfg = dir.join("config");
        let cdir = dir.join("cm");
        let changed = ensure_controlmaster_in(&cfg, &cdir).unwrap();
        assert!(changed);
        assert!(cfg.is_file());
    }
}
