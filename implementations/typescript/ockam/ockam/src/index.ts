import * as Ockam from "."

export * from "./worker";
export * from "./routing";
export * from "./node";

export class Hop implements Ockam.Worker {
  handleMessage(context: Ockam.Context, message: Ockam.Message) {
    console.log(context.address, " - received - ", message)
    // remove my address from beginning of onwardRoute
    message.onwardRoute.shift();
    // add my own address to beginning of returnRoute
    message.returnRoute.unshift(context.address);
    context.route(message)
  }
}

export class Printer implements Ockam.Worker {
  handleMessage(context: Ockam.Context, message: Ockam.Message) {
    console.log(context.address, " - received - ", message)
  }
}

export class Echoer implements Ockam.Worker {
  handleMessage(context: Ockam.Context, message: Ockam.Message) {
    console.log(context.address, " - received - ", message)
    // make returnRoute of incoming message, onwardRoute of outgoing message
    message.onwardRoute = message.returnRoute;
    // make my address the the returnRoute of the new message
    message.returnRoute = [context.address]
    context.route(message)
  }
}

export function example1() {
  let node = new Ockam.Node()
  node.startWorker("printer", new Printer())
  node.route({ onwardRoute: ["printer"], returnRoute: [], payload: "hello" })
}

export function example2() {
  let node = new Ockam.Node()

  node.startWorker("printer", new Printer())
  node.startWorker("h1", new Hop())
  node.startWorker("h2", new Hop())
  node.startWorker("h3", new Hop())

  node.route({ onwardRoute: ["h1", "h2", "h3", "printer"], returnRoute: [], payload: "hello" })
}

export function example3() {
  let node = new Ockam.Node()

  node.startWorker("echoer", new Echoer())
  node.startWorker("h1", new Hop())
  node.startWorker("h2", new Hop())
  node.startWorker("h3", new Hop())

  node.startWorker("app", (context: Ockam.Context, message: Ockam.Message) => {
    console.log(context.address, " - received - ", message)
  })

  node.route({ onwardRoute: ["h1", "h2", "h3", "echoer"], returnRoute: ["app"], payload: "hello" })
}

example3();
