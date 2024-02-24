mod bun_bin;

#[cfg(test)]
mod tests;

use eyre::{eyre, Result};
use git2::{build::RepoBuilder, Cred, FetchOptions, Index, PushOptions, RemoteCallbacks, Repository};
use regex::Regex;
use data_encoding::HEXLOWER;
use serde_json::Value;
use structstruck::strike;
use reqwest::Client;

async fn get_aur_version(client: &Client, pkgs: Vec<&str>) -> Result<Vec<String>> {
    let res = client
        .get(format!(
            "https://aur.archlinux.org/rpc/v5/info?arg[]={}",
            pkgs.join("&arg[]=")
        ))
        .send()
        .await?;
    let res_json: Value = res.json().await?;
    let json_walk = |idx: usize, value: &Value| -> Option<String> {
        let version = value.pointer(&format!("/results/{}/Version", idx))?.as_str()?.replacen("-1", "", 1);
        Some(version)
    };
    let mut versions = vec![];
    for idx in 0..pkgs.len() {
        let ver = json_walk(idx,&res_json).ok_or_else(|| eyre!("Cannot parse RPC response"))?;
        versions.push(ver);
    };
    Ok(versions)
}

fn ascii_to_val(encoded_key: String) -> Result<String> {
    let decoded_bytes = HEXLOWER.decode(encoded_key.as_bytes())?;
    Ok(std::str::from_utf8(&decoded_bytes)?.to_string())
}

fn commit_and_push(pk: &str, k: &str, name: &str, ext_version: &String, repo: &Repository, index: &mut Index) -> Result<()> {
    let mut config = repo.config()?;
    config.set_str("user.name", "Noel Jacob")?;
    let email = pk.split_whitespace().last()
        .ok_or_else(|| eyre!("No email"))?;
    config.set_str("user.email", email)?;
    let sig = repo.signature()?;
    let oid = index.write_tree()?;
    let parent_commit = match repo.head() {
        Ok(head) => repo.find_commit(head.target().ok_or_else(|| eyre!("No target"))?)?,
        Err(e) => return Err(e.into()),
    };
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        ext_version,
        &repo.find_tree(oid)?,
        &[&parent_commit],
    )?;
    let mut po = PushOptions::new();
    let mut cb = RemoteCallbacks::new();
    cb.credentials(move |_, _, _| Cred::ssh_key_from_memory("aur", Some(pk), k, None));
    po.remote_callbacks(cb);
    repo.find_remote("origin")?
        .push(&["refs/heads/master:refs/heads/master"], Some(&mut po))?;
    println!("{} updated to {}", name, ext_version);
    Ok(())
}

fn setup_git_and_get_keys<'a>() -> Result<(String, String, RepoBuilder<'a>)> {
    let mut fo = FetchOptions::new();
    let mut cb = RemoteCallbacks::new();
    let pk = ascii_to_val(std::env::var("SSH_PUB")?)?;
    let k = ascii_to_val(std::env::var("SSH_KEY")?)?;
    let pk1 = pk.clone();
    let k1 = k.clone();
    cb.credentials(move |_, _, _| Cred::ssh_key_from_memory("aur", Some(&pk1), &k1, None));
    fo.remote_callbacks(cb);
    fo.depth(1);
    let mut repo_client = RepoBuilder::new();
    repo_client.fetch_options(fo);
    Ok((pk, k, repo_client))
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();
    let pkgs = [
        bun_bin::Meta::new(&client).await?,
    ];
    let pkg_names = pkgs.iter().map(|p| p.name).collect();
    let aur_versions = get_aur_version(&client, pkg_names).await?;

    let (pk,  k, mut repo_client) = setup_git_and_get_keys()?;

    for (idx, pkg) in pkgs.iter().enumerate() {
        let ext_version = pkg.extern_version()?;
        if aur_versions[idx] == ext_version {
            println!("{} is up to date", pkg.name);
            continue;
        }
        let repo = repo_client
            .clone(&format!("ssh://aur@aur.archlinux.org/{}.git", pkg.name),
                std::env::current_dir()?.join(pkg.name).as_path(),
            )?;
        let mut index = repo.index()?;
        let dir = repo.workdir()
            .ok_or_else(|| eyre!("No workdir"))?;

        let replace_list = pkg.replace_list(&client).await?;
        for replace in replace_list {
            let mut file = std::fs::read_to_string(dir.join(&replace.filename))?;
            for (regex, value) in replace.regex {
                let re = Regex::new(&regex)?;
                file = re.replace(&file, value).to_string();
            }
            std::fs::write(dir.join(&replace.filename), file)?;
            index.add_path(replace.filename.as_ref())?;
        }

        commit_and_push(&pk, &k, pkg.name, &ext_version, &repo, &mut index)?;
    }
    Ok(())
}
