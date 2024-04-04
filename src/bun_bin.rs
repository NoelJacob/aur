use super::*;

strike! {
        #[strikethrough[derive(serde::Deserialize, Debug)]]
        struct GithubRelease {
            tag_name: String,
            assets: Vec<struct {
                name: String,
                browser_download_url: String,
                }>,
        }
}

pub struct BunBin {
   // The package name in AUR
   pub pkg_name: String,
   github_release: GithubRelease,
}

impl BunBin {
    // Setup new instance
    pub async fn new(client: &Client) -> Result<Self> {
        let github_repo = "oven-sh/bun";
        let url = format!(
            "https://api.github.com/repos/{}/releases/latest",
            github_repo
        );

        #[cfg(not(debug_assertions))]
        let res = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header( "Authorization",format!("Bearer {}", std::env::var("GITHUB_TOKEN")?))
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "NoelJacob")
            .send().await?;

        #[cfg(debug_assertions)]
            let res = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "NoelJacob")
            .send().await?;

        Ok(Self {
            github_release: res.json().await?,
            pkg_name: "bun-bin".to_string(),
        })
    }

    // Get external version
    pub fn extern_version(&self) -> Result<String> {
        let github_version = self.github_release
            .tag_name
            .as_str()
            .replacen("bun-v", "", 1);
        Ok(github_version)
    }

    // Return list of files with regex to replace
    pub async fn replace_list(&self, client: &Client) -> Result<[Replaces; 2]> {
        let sha = get_sha(client, &self.github_release).await?;
        let list = [
            Replaces {
                filename: ".SRCINFO".to_string(),
                regex: vec![
                    (r"pkgver = [0-9\.]+".to_string(), format!("pkgver = {}", self.extern_version()?)),
                    (r"sha256sums_aarch64 = [0-9a-fA-F]+".to_string(), format!("sha256sums_aarch64 = {}", sha.aarch.clone())),
                    (r"sha256sums_x86_64 = [0-9a-fA-F]+".to_string(), format!("sha256sums_x86_64 = {}",sha.x64.clone()))]
            },
            Replaces {
                filename: "PKGBUILD".to_string(),
                regex: vec![
                    (r"pkgver=[0-9\.]+".to_string(), format!("pkgver={}", self.extern_version()?)),
                    (r"sha256sums_aarch64=\('[0-9a-fA-F]+'".to_string(), format!("sha256sums_aarch64=('{}'", sha.aarch)),
                    (r"sha256sums_x86_64=\('[0-9a-fA-F]+'".to_string(), format!("sha256sums_x86_64=('{}'", sha.x64)),
                    (r"_baseline_sha256sums='[0-9a-fA-F]+'".to_string(), format!("_baseline_sha256sums='{}'", sha.baseline))],
            }
        ];

        Ok(list)
    }

}

pub struct Replaces {
    pub filename: String,
    pub regex: Vec<(String, String)>,
}


struct Sha {
    aarch: String,
    x64: String,
    baseline: String,
}

async fn get_sha(client: &Client, github_release: &GithubRelease) -> Result<Sha> {
    let sha_url = &github_release
        .assets
        .iter()
        .find(|a| a.name == "SHASUMS256.txt")
        .ok_or_else(|| eyre!("No checksums"))?
        .browser_download_url;
    let res = client.get(sha_url).send().await?;
    let sha_txt = res.text().await?;
    fn extract(shas: &str, name: &str) -> Option<String> {
        let value = shas
            .lines()
            .find(|x| x.contains(name))?
            .split_whitespace()
            .next()?
            .to_string();
        Some(value)
    }
    let sha = Sha {
        aarch: extract(&sha_txt,"bun-linux-aarch64.zip").ok_or_else(|| eyre!("No aarch64"))?,
        x64: extract(&sha_txt, "bun-linux-x64.zip").ok_or_else(|| eyre!("No x64"))?,
        baseline: extract(&sha_txt,"bun-linux-x64-baseline.zip").ok_or_else(|| eyre!("No baseline"))?,
    };
    Ok(sha)
}