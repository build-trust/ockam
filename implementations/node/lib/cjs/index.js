"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Ockam = exports.Relay = exports.TcpOutlet = exports.TcpInlet = exports.Policy = exports.Node = exports.Identity = exports.Project = void 0;
const child_process_1 = require("child_process");
class Command {
    static exec(command, args) {
        return __awaiter(this, void 0, void 0, function* () {
            let out;
            let errMsg;
            return new Promise((resolve, reject) => {
                console.log(`Running \`${command} ${args.join(" ")}\``);
                const cmd = (0, child_process_1.spawn)(command, args);
                cmd.stdout.on("data", (data) => {
                    out = data;
                });
                cmd.stderr.on("data", (data) => {
                    console.log(data);
                });
                cmd.on('error', (error) => {
                    console.log(`error: ${error.message}`);
                });
                cmd.on("close", (code) => {
                    if (code != 0) {
                        let err = new Error(`\`${command} ${args.join(" ")}\` return exit code ${code}`);
                        reject(err);
                    }
                    else {
                        resolve(out);
                    }
                });
            });
        });
    }
}
class Identity {
    constructor(name) {
        this.name = name;
    }
    create() {
        return __awaiter(this, void 0, void 0, function* () {
            return yield Command.exec('ockam', ["identity", "create", this.name]);
        });
    }
}
exports.Identity = Identity;
class Node {
    constructor(name, identity) {
        this.name = name;
        this.identity = identity;
    }
    create() {
        return __awaiter(this, void 0, void 0, function* () {
            return yield Command.exec("ockam", ["node", "create", this.name, `--identity=${this.identity.name}`]);
        });
    }
}
exports.Node = Node;
class Policy {
    constructor(at, resource, expression) {
        this.at = at;
        this.resource = resource;
        this.expression = expression;
    }
    create() {
        return __awaiter(this, void 0, void 0, function* () {
            return yield Command.exec("ockam", ["policy", "create", `--at=${this.at}`, `--resource=${this.resource}`, `--expression=${this.expression}`]);
        });
    }
}
exports.Policy = Policy;
class TcpInlet {
    constructor(at, from, to) {
        this.at = at;
        this.from = from;
        this.to = to;
    }
    create() {
        return __awaiter(this, void 0, void 0, function* () {
            return yield Command.exec("ockam", ["tcp-inlet", "create", `--at=/node/${this.at}`, `--from=${this.from}`, `--to=${this.to}`]);
        });
    }
}
exports.TcpInlet = TcpInlet;
class TcpOutlet {
    constructor(at, from, to) {
        this.at = at;
        this.from = from;
        this.to = to;
    }
    create() {
        return __awaiter(this, void 0, void 0, function* () {
            return yield Command.exec("ockam", ["tcp-outlet", "create", `--at=/node/${this.at}`, `--from=${this.from}`, `--to=${this.to}`]);
        });
    }
}
exports.TcpOutlet = TcpOutlet;
class Relay {
    constructor(name, project, outlet) {
        this.project = project;
        this.name = name;
        this.at = outlet.at;
        this.from = outlet.from;
        this.outlet = `/service/forward_to_${this.name}/secure/api${this.from}`;
    }
    create() {
        return __awaiter(this, void 0, void 0, function* () {
            return yield Command.exec("ockam", ["relay", "create", this.name, `--at=${this.project.name}`, `--to=/node/${this.at}`]);
        });
    }
}
exports.Relay = Relay;
class Project {
    constructor() {
        this.name = "default";
    }
    enroll(attributes) {
        return __awaiter(this, void 0, void 0, function* () {
            return yield Command.exec("ockam", ["project", "enroll", "--attribute", attributes]);
        });
    }
    authenticate(token, identity) {
        return __awaiter(this, void 0, void 0, function* () {
            return yield Command.exec("ockam", ["project", "authenticate", token, `--identity=${identity.name}`]);
        });
    }
}
exports.Project = Project;
class Ockam {
    reset() {
        return __awaiter(this, void 0, void 0, function* () {
            return yield Command.exec("ockam", ["reset", "-y"]);
        });
    }
    pbcopy(data) {
        var proc = (0, child_process_1.spawn)('pbcopy');
        proc.stdin.write(data);
        proc.stdin.end();
    }
    enroll() {
        return __awaiter(this, void 0, void 0, function* () {
            let otc;
            return new Promise((resolve, reject) => {
                console.log("Running `ockam enroll`");
                const cmd = (0, child_process_1.spawn)("ockam", ["enroll"]);
                cmd.stdout.on("data", (data) => {
                });
                cmd.stderr.on("data", (data) => {
                    console.log(`stderr: ${data}`);
                    const OTC_REGEX = /First copy your one-time code:.+?([A-Z]{4}-[A-Z]{4})/;
                    const matched = data.toString().match(OTC_REGEX);
                    if (matched) {
                        otc = matched[1];
                        this.pbcopy(otc);
                    }
                    if (data.toString().match(/Then press enter to open/gm)) {
                        cmd.stdin.write("\n");
                    }
                });
                cmd.on('error', (error) => {
                    console.log(`error: ${error.message}`);
                });
                cmd.on("close", (code) => {
                    console.log(`child process exited with code ${code}`);
                    if (code === 0)
                        resolve(true);
                    reject();
                });
            });
        });
    }
    connectTunnel(ticket, port, relayOutlet) {
        return __awaiter(this, void 0, void 0, function* () {
            const identity = new Identity("localid");
            yield identity.create();
            const project = new Project;
            yield project.authenticate(ticket, identity);
            const node = new Node("localnode", identity);
            yield node.create();
            const policy = new Policy(node.name, 'tcp-inlet', '(= subject.component "influxdb")');
            yield policy.create();
            const inlet = new TcpInlet(node.name, `127.0.0.1:${port}`, relayOutlet);
            yield inlet.create();
        });
    }
}
exports.Ockam = Ockam;
