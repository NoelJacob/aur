// index.ts
import {existsSync, mkdirSync, writeFileSync} from "fs";
import {execSync} from "child_process";
import {env, exit} from "process";
if (!existsSync(`${env["HOME"]}/.ssh/aur_ed25519`)) {
  if (!env["SSH_KEY"])
    throw new Error("SSH_KEY not set");
  if (!existsSync(`${env["HOME"]}/.ssh`))
    mkdirSync(`${env["HOME"]}/.ssh`, { recursive: true });
  writeFileSync(`${env["HOME"]}/.ssh/aur_ed25519`, env["SSH_KEY"], { encoding: "utf-8" });
}
var s = execSync("ssh-agent -s").toString() + `ssh-add ${env["HOME"]}/.ssh/aur_ed25519;`;
execSync(s + "git submodule update --init --recursive");
var checkBun = () => {
};
checkBun();
exit();
