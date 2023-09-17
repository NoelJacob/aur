use eyre::{eyre, Result};
use git2::{build::RepoBuilder, Cred, FetchOptions, PushOptions, RemoteCallbacks};
use regex::Regex;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let mut fo = FetchOptions::new();
    let mut cb = RemoteCallbacks::new();
    let pk = std::env::var("SSH_PUB")?;
    let k = format!(
        "-----BEGIN OPENSSH PRIVATE KEY-----\n{}\n-----END OPENSSH PRIVATE KEY-----",
        std::env::var("SSH_KEY")?.replace(' ', "\n")
    );
    let fo_pk = pk.clone();
    let fo_key = k.clone();
    cb.credentials(move |_, _, _| Cred::ssh_key_from_memory("aur", Some(&fo_pk), &fo_key, None));
    fo.remote_callbacks(cb);
    let mut repo_client = RepoBuilder::new();
    repo_client.fetch_options(fo);

    let client = reqwest::Client::new();

    // aur
    let pkg = "bunjs-bin";
    let res = client
        .get(format!(
            "https://aur.archlinux.org/rpc/?v=5&type=info&arg[]={}",
            pkg
        ))
        .send()
        .await?;
    let res_json: serde_json::Value = res.json().await?;
    let aur_version = res_json
        .pointer("/results/0/Version")
        .ok_or_else(|| eyre!("No version"))?
        .as_str()
        .ok_or_else(|| eyre!("Version not string"))?
        .replacen("-1", "", 1);

    structstruck::strike!(
        #[strikethrough[derive(serde::Deserialize, Debug)]]
        struct GithubRelease {
            tag_name: String,
            assets: Vec<struct {
                name: String,
                browser_download_url: String,
                }>,
        }
    );
    // release
    let github_repo = "oven-sh/bun";
    let res = client
        .get(format!(
            "https://api.github.com/repos/{}/releases/latest",
            github_repo
        ))
        .header("Accept", "application/vnd.github+json")
        .header( "Authorization",format!("Bearer {}", std::env::var("GITHUB_TOKEN")?))
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "NoelJacob")
        .send()
        .await?;
    let release_json: GithubRelease = res.json().await?;
    let github_version = release_json.tag_name.replacen("bun-v", "", 1);

    // check and update
    if github_version == aur_version {
        println!("{} is up to date", pkg);
        return Ok(());
    }

    // get checksums
    let sha_url = &release_json
        .assets
        .iter()
        .find(|a| a.name == "SHASUMS256.txt")
        .ok_or_else(|| eyre!("No checksums"))?
        .browser_download_url;
    let res = client.get(sha_url).send().await?;
    let sha_txt = res.text().await?;

    let get_sha = |name: &str| -> Result<&str> {
        sha_txt
            .lines()
            .find(|x| x.contains(name))
            .ok_or_else(|| eyre!("No checksum for {}", name))?
            .split_whitespace()
            .next()
            .ok_or_else(|| eyre!("No checksum for {}", name))
    };
    struct Sha<'a> {
        aarch: &'a str,
        x64: &'a str,
        baseline: &'a str,
    }
    let sha = Sha {
        aarch: get_sha("bun-linux-aarch64.zip")?,
        x64: get_sha("bun-linux-x64.zip")?,
        baseline: get_sha("bun-linux-x64-baseline.zip")?,
    };

    // clone repo
    let repo = repo_client.clone(
        &format!("ssh://aur@aur.archlinux.org/{}.git", pkg),
        std::env::current_dir()?.join(pkg).as_path(),
    )?;
    let dir = repo.workdir().ok_or_else(|| eyre!("No workdir"))?;

    // update pkgbuild and srcinfo
    let mut pkgbuild = std::fs::read_to_string(dir.join("PKGBUILD"))?;
    let mut srcinfo = std::fs::read_to_string(dir.join(".SRCINFO"))?;
    // update version
    pkgbuild = pkgbuild.replacen(&aur_version, &github_version, 1);
    srcinfo = srcinfo.replacen(&aur_version, &github_version, 1);
    // update shaaarch
    let mut re = Regex::new(r"sha256sums_aarch64=\('[0-9a-z]+'\)")?;
    pkgbuild = re
        .replace(&pkgbuild, format!("sha256sums_aarch64=('{}')", sha.aarch))
        .to_string();
    re = Regex::new(r"sha256sums_aarch64 = [0-9a-z]+")?;
    srcinfo = re
        .replacen(&srcinfo, 1, format!("sha256sums_aarch64 = {}", sha.aarch))
        .to_string();
    // update shax64
    re = Regex::new(r"sha256sums_x86_64=\('[0-9a-z]+'\)")?;
    pkgbuild = re
        .replace(&pkgbuild, format!("sha256sums_x86_64=('{}')", sha.x64))
        .to_string();
    re = Regex::new(r"sha256sums_x86_64 = [0-9a-z]+")?;
    srcinfo = re
        .replace(&srcinfo, format!("sha256sums_x86_64 = {}", sha.x64))
        .to_string();
    // update shabaseline
    re = Regex::new(r"_baseline_sha256sums='[0-9a-z]+'")?;
    pkgbuild = re
        .replace(
            &pkgbuild,
            format!("_baseline_sha256sums='{}'", sha.baseline),
        )
        .to_string();
    // write
    std::fs::write(dir.join("PKGBUILD"), pkgbuild)?;
    std::fs::write(dir.join(".SRCINFO"), srcinfo)?;

    // commit and push
    let mut config = repo.config()?;
    config.set_str("user.name", "Noel Jacob")?;
    let email = pk
        .split_whitespace()
        .last()
        .ok_or_else(|| eyre!("No email"))?;
    config.set_str("user.email", email)?;
    let sig = repo.signature()?;
    let mut index = repo.index()?;
    index.add_path(Path::new("PKGBUILD"))?;
    index.add_path(Path::new(".SRCINFO"))?;
    let oid = index.write_tree()?;
    let parent_commit = match repo.head() {
        Ok(head) => repo.find_commit(head.target().ok_or_else(|| eyre!("No target"))?)?,
        Err(e) => return Err(e.into()),
    };
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &github_version,
        &repo.find_tree(oid)?,
        &[&parent_commit],
    )?;
    let mut po = PushOptions::new();
    let mut cb = RemoteCallbacks::new();
    cb.credentials(move |_, _, _| Cred::ssh_key_from_memory("aur", Some(&pk), &k, None));
    po.remote_callbacks(cb);
    repo.find_remote("origin")?
        .push(&["refs/heads/master:refs/heads/master"], Some(&mut po))?;
    println!("{} updated to {}", pkg, github_version);

    Ok(())
}
