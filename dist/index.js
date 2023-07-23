// index.ts
import {existsSync, mkdirSync, writeFileSync, readFileSync} from "fs";
import {execSync} from "child_process";
import {env, exit} from "process";
if (!existsSync(`${env["HOME"]}/.ssh/aur_ed25519`)) {
  if (!env["SSH_KEY"])
    throw new Error("SSH_KEY not set");
  if (!existsSync(`${env["HOME"]}/.ssh`))
    mkdirSync(`${env["HOME"]}/.ssh`, { recursive: true, mode: 448 });
  writeFileSync(`${env["HOME"]}/.ssh/aur_ed25519`, env["SSH_KEY"], { encoding: "utf-8", mode: 384 });
  execSync("ssh-keyscan aur.archlinux.org >> ~/.ssh/known_hosts");
}
var s = execSync("ssh-agent -s").toString() + `ssh-add ${env["HOME"]}/.ssh/aur_ed25519 &&\n`;
execSync(s + "git submodule update --init --recursive");
var checkBun = async () => {
  let p = readFileSync("bunjs-bin/.SRCINFO", { encoding: "utf-8" });
  let x = p.match(/pkgver = ([0-9.]+)/);
  if (!x)
    throw new Error("No version");
  let v1 = x[1];
  let pbase = readFileSync("bunjs-baseline-bin/.SRCINFO", { encoding: "utf-8" });
  let xbase = p.match(/pkgver = ([0-9.]+)/);
  if (!xbase)
    throw new Error("No version");
  let v1base = xbase[1];
  let l = await fetch("https://api.github.com/repos/oven-sh/bun/releases/latest", {
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${env["GITHUB_TOKEN"]}`,
      "X-GitHub-Api-Version": "2022-11-28"
    }
  }).then((x3) => x3.json());
  let x2 = l.tag_name.match(/bun-v([0-9.]+)/);
  if (!x2)
    throw new Error("Cannot regex upstream version");
  let v2 = x2[1];
  if (v1 !== v2) {
    let shax862 = "", shaarm2 = "";
    for (let x5 of l.assets) {
      if (x5.name === "SHASUMS256.txt") {
        let res = await fetch(x5.browser_download_url).then((x6) => x6.text());
        let shas = res.split("\n");
        for (let x6 of shas) {
          let y = x6.split("  ");
          if (y[1] === "bun-linux-x64.zip")
            shax862 = y[0];
          else if (y[1] === "bun-linux-aarch64.zip")
            shaarm2 = y[0];
        }
        break;
      }
    }
    let x3 = p.match(/sha256sums_x86_64 = ([0-9a-z]+)/);
    if (!x3)
      throw new Error("No sha256sums_x86_64");
    let shax861 = x3[1];
    let x4 = p.match(/sha256sums_aarch64 = ([0-9a-z]+)/);
    if (!x4)
      throw new Error("No sha256sums_aarch64");
    let shaarm1 = x4[1];
    let pkg1 = readFileSync("bunjs-bin/PKGBUILD", { encoding: "utf-8" });
    let pkg2 = pkg1.replaceAll(v1, v2).replaceAll(shax861, shax862).replaceAll(shaarm1, shaarm2);
    writeFileSync("bunjs-bin/PKGBUILD", pkg2, { encoding: "utf-8" });
    let src1 = readFileSync("bunjs-bin/.SRCINFO", { encoding: "utf-8" });
    let src2 = src1.replaceAll(v1, v2).replaceAll(shax861, shax862).replaceAll(shaarm1, shaarm2);
    writeFileSync("bunjs-bin/.SRCINFO", src2, { encoding: "utf-8" });
    execSync("git add PKGBUILD .SRCINFO", { cwd: "bunjs-bin" });
    execSync(`git commit -m "${v2}"`, { cwd: "bunjs-bin" });
    execSync(s + "git push", { cwd: "bunjs-bin" });
  }
  if (v1base !== v2) {
    let sha2base = "";
    for (let x4 of l.assets) {
      if (x4.name === "SHASUMS256.txt") {
        let res = await fetch(x4.browser_download_url).then((x5) => x5.text());
        let shas = res.split("\n");
        for (let x5 of shas) {
          let y = x5.split("  ");
          if (y[1] === "bun-linux-x64-baseline.zip") {
            sha2base = y[0];
            break;
          }
        }
        break;
      }
    }
    let x3 = pbase.match(/sha256sums = ([0-9a-z]+)/);
    if (!x3)
      throw new Error("No sha256sums");
    let sha1base = x3[1];
    let pkg1base = readFileSync("bunjs-baseline-bin/PKGBUILD", { encoding: "utf-8" });
    let pkg2base = pkg1base.replaceAll(v1base, v2).replaceAll(sha1base, sha2base);
    writeFileSync("bunjs-baseline-bin/PKGBUILD", pkg2base, { encoding: "utf-8" });
    let src1base = readFileSync("bunjs-baseline-bin/.SRCINFO", { encoding: "utf-8" });
    let src2base = src1base.replaceAll(v1base, v2).replaceAll(sha1base, sha2base);
    writeFileSync("bunjs-baseline-bin/.SRCINFO", src2base, { encoding: "utf-8" });
    execSync("git add PKGBUILD .SRCINFO", { cwd: "bunjs-baseline-bin" });
    execSync(`git commit -m "${v2}"`, { cwd: "bunjs-baseline-bin" });
    execSync(s + "git push", { cwd: "bunjs-baseline-bin" });
  }
};
await checkBun();
exit();
