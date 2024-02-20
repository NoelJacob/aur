use super::*;

#[tokio::test]
async fn test_get_aur_version() -> Result<()> {
    let client = reqwest::Client::new();
    let pkgs = vec!["bun-bin", "bun-git"];
    let aur_version = get_aur_version(&client, pkgs).await?;
    assert_eq!(aur_version, [
        "1.0.28",
        "1.0.2.r20.bab9889",
    ]);
    Ok(())
}

#[tokio::test]
async fn bun_bin_extern_version() -> Result<()> {
    let client = reqwest::Client::new();
    let bb = bun_bin::Meta::new(&client).await?;
    let github_version = bb.extern_version().await?;
    assert_eq!(github_version, "1.0.28");
    Ok(())
}