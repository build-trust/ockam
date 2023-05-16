import { spawn } from "child_process";
class Command {
    static async exec(command, args) {
        let out;
        let errMsg;
        return new Promise((resolve, reject) => {
            console.log(`Running \`${command} ${args.join(" ")}\``);
            const cmd = spawn(command, args);
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
    }
}
class Identity {
    name;
    constructor(name) {
        this.name = name;
    }
    async create() {
        return await Command.exec('ockam', ["identity", "create", this.name]);
    }
}
class Node {
    name;
    identity;
    constructor(name, identity) {
        this.name = name;
        this.identity = identity;
    }
    async create() {
        return await Command.exec("ockam", ["node", "create", this.name, `--identity=${this.identity.name}`]);
    }
}
class Policy {
    at;
    resource;
    expression;
    constructor(at, resource, expression) {
        this.at = at;
        this.resource = resource;
        this.expression = expression;
    }
    async create() {
        return await Command.exec("ockam", ["policy", "create", `--at=${this.at}`, `--resource=${this.resource}`, `--expression=${this.expression}`]);
    }
}
class TcpInlet {
    at;
    from;
    to;
    constructor(at, from, to) {
        this.at = at;
        this.from = from;
        this.to = to;
    }
    async create() {
        return await Command.exec("ockam", ["tcp-inlet", "create", `--at=/node/${this.at}`, `--from=${this.from}`, `--to=${this.to}`]);
    }
}
class TcpOutlet {
    at;
    from;
    to;
    constructor(at, from, to) {
        this.at = at;
        this.from = from;
        this.to = to;
    }
    async create() {
        return await Command.exec("ockam", ["tcp-outlet", "create", `--at=/node/${this.at}`, `--from=${this.from}`, `--to=${this.to}`]);
    }
}
class Relay {
    outlet;
    project;
    name;
    at;
    from;
    constructor(name, project, outlet) {
        this.project = project;
        this.name = name;
        this.at = outlet.at;
        this.from = outlet.from;
        this.outlet = `/service/forward_to_${this.name}/secure/api${this.from}`;
    }
    async create() {
        return await Command.exec("ockam", ["relay", "create", this.name, `--at=${this.project.name}`, `--to=/node/${this.at}`]);
    }
}
class Project {
    name;
    constructor() {
        this.name = "default";
    }
    async enroll(attributes) {
        return await Command.exec("ockam", ["project", "enroll", "--attribute", attributes]);
    }
    async authenticate(token, identity) {
        return await Command.exec("ockam", ["project", "authenticate", token, `--identity=${identity.name}`]);
    }
}
class Ockam {
    async reset() {
        return await Command.exec("ockam", ["reset", "-y"]);
    }
    pbcopy(data) {
        var proc = spawn('pbcopy');
        proc.stdin.write(data);
        proc.stdin.end();
    }
    async enroll() {
        let otc;
        return new Promise((resolve, reject) => {
            console.log("Running `ockam enroll`");
            const cmd = spawn("ockam", ["enroll"]);
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
    }
    async connectTunnel(ticket, port, relayOutlet) {
        const identity = new Identity("localid");
        await identity.create();
        const project = new Project;
        await project.authenticate(ticket, identity);
        const node = new Node("localnode", identity);
        await node.create();
        const policy = new Policy(node.name, 'tcp-inlet', '(= subject.component "influxdb")');
        await policy.create();
        const inlet = new TcpInlet(node.name, `127.0.0.1:${port}`, relayOutlet);
        await inlet.create();
    }
}
export { Project, Identity, Node, Policy, TcpInlet, TcpOutlet, Relay, Ockam };
