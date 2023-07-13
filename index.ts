import {existsSync, mkdirSync, writeFileSync, readdirSync} from "fs";
import {execSync} from "child_process";
import {env, exit} from "process";

const c = execSync("ssh-agent -c").toString();
if (!existsSync(`${env["HOME"]}/.ssh/id_ed25519`)) {
    if (!env["SSH_KEY"]) throw new Error("SSH_KEY not set");
    if (!existsSync(`${env["HOME"]}/.ssh`)) mkdirSync(`${env["HOME"]}/.ssh`, {recursive: true});
    writeFileSync(`${env["HOME"]}/.ssh/id_ed25519`, env["SSH_KEY"], {encoding: "utf-8"});
}
execSync(c
    +`ssh-add ${env["HOME"]}/.ssh/id_ed25519`
);
execSync("git submodule update --init --recursive");

const checkBun = () => {

}

checkBun();
exit();