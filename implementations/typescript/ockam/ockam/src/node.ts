import { Address, Message, Router, LOCAL } from "./routing";
import { Worker, Context } from "./worker";

export type NodeWorkerAddress =
  | string
  | Uint8Array
  | [type: 0, value: string | Uint8Array];

export class NodeContext implements Context {
  address: Address;
  node: Node;

  constructor(address: string | Uint8Array, node: Node) {
    this.address = address;
    this.node = node;
  }

  route(message: Message) {
    this.node.route(message);
  }
}

export class Node {
  router: Router;
  workers: Record<
    string,
    { address: NodeWorkerAddress; worker: Worker; context: NodeContext }
  >;

  constructor() {
    let unroutableMessageHandler = (message: Message) => {
      console.error("could not route message: ", message);
    };

    let localAddressTypeRouterPlugin = {
      messageHandler: (message: Message) => {
        return this.handleRoutingMessage(message);
      },
      convertAddressToString: (address: Address) => {
        return this.convertAddressToString(address);
      },
      convertAddressToUint8Array: (address: Address) => {
        return this.convertAddressToUint8Array(address);
      },
    };

    let router = new Router(unroutableMessageHandler);
    router.registerPlugin(LOCAL, localAddressTypeRouterPlugin);

    this.router = router;
    this.workers = {};
  }

  startWorker(
    address: string | Uint8Array,
    worker: Worker | ((context: Context, message: Message) => void),
  ) {
    if (typeof worker === "function") {
      worker = { handleMessage: worker };
    }

    let context = new NodeContext(address, this);
    // TODO: setup
    let addressAsString = this.convertAddressToString(address);
    if (addressAsString) {
      this.workers[addressAsString] = { address, worker, context };
    } else {
      // TODO: error?
    }
  }

  route(message: Message) {
    this.router.route(message);
  }

  handleRoutingMessage(message: Message) {
    let firstAddress = message.onwardRoute[0];
    let firstAddressAsString = this.convertAddressToString(firstAddress);
    if (firstAddress && firstAddressAsString) {
      let stored = this.workers[firstAddress.toString()];
      setTimeout(() => {
        stored.worker.handleMessage(stored.context, message);
      });
    } else {
      console.error("could not route message", message);
    }
  }

  convertAddressToString(address: Address) {
    // if address is a tuple [LOCAL, address], make address[1] the address.
    if (Array.isArray(address) && address.length === 2 && address[0] === LOCAL)
      address = address[1];

    // if address is already a string, return it
    if (typeof address === "string") return address;

    if (Array.isArray(address)) {
      let output: Array<string> = [];
      for (var i = 0; i < address.length; i++) {
        let item = address[i];
        if (typeof item === "number" && item >= 0 && item <= 255) {
          let o = item.toString(16);
          output[1] = o.length === 2 ? o : "0" + o;
        } else {
          return undefined;
        }
      }
    }

    return undefined;
  }

  convertAddressToUint8Array(address: Address) {
    return new Uint8Array(0);
  }
}
