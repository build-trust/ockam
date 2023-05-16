declare class Identity {
    name: string;
    constructor(name: string);
    create(): Promise<unknown>;
}
declare class Node {
    name: string;
    private identity;
    constructor(name: string, identity: Identity);
    create(): Promise<unknown>;
}
declare class Policy {
    private at;
    private resource;
    private expression;
    constructor(at: string, resource: string, expression: string);
    create(): Promise<unknown>;
}
declare class TcpInlet {
    private at;
    private from;
    private to;
    constructor(at: string, from: string, to: string);
    create(): Promise<unknown>;
}
declare class TcpOutlet {
    at: string;
    from: string;
    to: string;
    constructor(at: string, from: string, to: string);
    create(): Promise<unknown>;
}
declare class Relay {
    outlet: string;
    private project;
    private name;
    private at;
    private from;
    constructor(name: string, project: Project, outlet: TcpOutlet);
    create(): Promise<unknown>;
}
declare class Project {
    name: string;
    constructor();
    enroll(attributes: string): Promise<unknown>;
    authenticate(token: string, identity: Identity): Promise<unknown>;
}
declare class Ockam {
    reset(): Promise<unknown>;
    pbcopy(data: string): void;
    enroll(): Promise<unknown>;
    connectTunnel(ticket: string, port: string, relayOutlet: string): Promise<void>;
}
export { Project, Identity, Node, Policy, TcpInlet, TcpOutlet, Relay, Ockam };
//# sourceMappingURL=index.d.ts.map