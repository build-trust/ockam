import * as native from "@ockam/nodejs_native";

export function hello(name: string): string {
  return native.hello(name);
}
