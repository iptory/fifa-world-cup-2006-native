use fifa_game_data::prepared_game;

#[test]
fn missing_manifest_is_first_run() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    assert!(prepared_game(directory.path())?.is_none());
    Ok(())
}

#[test]
fn corrupt_manifest_reports_an_error() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    std::fs::write(directory.path().join("fifa-game-data.json"), b"{not json")?;
    assert!(prepared_game(directory.path()).is_err());
    Ok(())
}
