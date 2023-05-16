import { Ockam, Project, Identity, Policy, TcpOutlet, Relay, Node } from "../lib/esm/index.mjs"

const ockam = new Ockam()
await ockam.reset()
await ockam.enroll()
const project = new Project()
const telegraficket = await project.enroll("component=telegraf")
const influxticket = await project.enroll("component=influxdb")

const identity = new Identity("influxdb")
await identity.create()
await project.authenticate(influxticket, identity)
const node = new Node("influxdb", identity)
await node.create()
const policy = new Policy(node.name, 'tcp-outlet', '(= subject.component "telegraf")')
await policy.create()


const outlet = new TcpOutlet(node.name, "/service/outlet", "127.0.0.1:8086") 
await outlet.create() 
const relay = new Relay(node.name, project, outlet.)
await relay.create()

const localPort = "8087"
await ockam.connectTunnel(telegraficket, localPort, relay.outlet)