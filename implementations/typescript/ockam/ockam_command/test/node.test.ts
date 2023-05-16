import { describe, expect, test } from "@jest/globals";
import { Node } from "../src/node";

describe("Node", () => {
  test("Can be created and deleted", async () => {
    let node = await Node.create("hello");
    expect(node).not.toBeNull();

    if (node) {
      let isRunning = await node.isRunning();
      expect(isRunning).toBe(true);

      await node.delete();
      isRunning = await node.isRunning();
      expect(isRunning).toBe(false);
    }
  });
});
