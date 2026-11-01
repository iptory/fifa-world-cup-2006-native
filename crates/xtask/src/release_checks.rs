use std::fs;

#[test]
fn release_rejects_generated_game_source() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let generated = root.path().join("project/native/generated/default");
    fs::create_dir_all(&generated)?;
    fs::write(generated.join("game_recomp.cpp"), b"private local output")?;
    assert!(super::audit(root.path(), &root.path().join("project")).is_err());
    Ok(())
}

#[test]
fn release_rejects_disc_images_and_unapproved_executables() -> anyhow::Result<()> {
    for filename in [
        "disc.iso",
        "default.xex",
        "fifa_world_cup_2006.exe",
        "unknown.exe",
        "signing.key",
    ] {
        let root = tempfile::tempdir()?;
        fs::write(root.path().join(filename), b"must remain local")?;
        assert!(super::audit(root.path(), &root.path().join("project")).is_err());
    }
    Ok(())
}
