import { Address, Message } from "./routing";

export interface Context {
  address: Address;
  route: (message: Message) => any;
}

export interface Worker {
  setup?: (options: Record<string, any>) => any;
  handleMessage: (context: Context, message: Message) => void;
}
