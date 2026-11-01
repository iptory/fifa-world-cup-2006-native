use std::path::{Path, PathBuf};

pub(crate) fn join_inside(root: &Path, rel: &str) -> anyhow::Result<PathBuf> {
    if rel.is_empty() || rel.starts_with('/') {
        anyhow::bail!("ISO path must be nonempty and relative ({rel:?})");
    }
    let mut out = PathBuf::from(root);
    for part in rel.split('/').filter(|s| !s.is_empty()) {
        push_component(&mut out, part)?;
    }
    if out == root {
        anyhow::bail!("empty ISO path");
    }
    Ok(out)
}

fn push_component(out: &mut PathBuf, part: &str) -> anyhow::Result<()> {
    if part == "." || part == ".." {
        anyhow::bail!("ISO path escapes image ({part})");
    }
    if part.len() > 255 || part.bytes().any(|b| b == 0) {
        anyhow::bail!("bad ISO path component");
    }
    for c in part.chars() {
        if c.is_control() || "<>:\"\\|?*".contains(c) {
            anyhow::bail!("bad ISO path component {part:?}");
        }
    }
    if part.ends_with([' ', '.']) {
        anyhow::bail!("bad Windows path component {part:?}");
    }
    if is_reserved_windows(&part.to_ascii_uppercase()) {
        anyhow::bail!("reserved Windows path component {part:?}");
    }
    out.push(part);
    Ok(())
}

fn is_reserved_windows(upper: &str) -> bool {
    let stem = upper.split('.').next().unwrap_or(upper);
    matches!(
        stem,
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

#[cfg(test)]
mod tests {
    use super::join_inside;
    use std::path::Path;

    #[test]
    fn accepts_normal_relative_path() -> anyhow::Result<()> {
        let root = Path::new("/tmp/stage");
        let joined = join_inside(root, "data/file.big")?;
        assert_eq!(joined, root.join("data").join("file.big"));
        Ok(())
    }

    #[test]
    fn rejects_dotdot_escape() {
        assert!(join_inside(Path::new("/tmp/stage"), "../evil").is_err());
    }

    #[test]
    fn rejects_absolute_escape() {
        assert!(join_inside(Path::new("/tmp/stage"), "/abs").is_err());
    }

    #[test]
    fn rejects_windows_reserved_name() {
        assert!(join_inside(Path::new("/tmp/stage"), "aux/x").is_err());
    }

    #[test]
    fn rejects_trailing_dot() {
        assert!(join_inside(Path::new("/tmp/stage"), "name./x").is_err());
    }

    #[test]
    fn rejects_empty_path() {
        assert!(join_inside(Path::new("/tmp/stage"), "/").is_err());
    }

    #[test]
    fn unicode_components_do_not_split_utf8() -> anyhow::Result<()> {
        let root = Path::new("stage");
        assert_eq!(
            join_inside(root, "é日/file.big")?,
            root.join("é日/file.big")
        );
        Ok(())
    }

    #[test]
    fn windows_device_names_remain_reserved_with_extensions() {
        assert!(join_inside(Path::new("stage"), "con.txt").is_err());
    }
}
