import { Runner } from "./runner";

export class Node {
  name: string;

  constructor(name: string, private runner: Runner = new Runner()) {
    this.name = name;
  }

  async show(): Promise<boolean> {
    try {
      const { code } = await this.runner.run(["node", "show", this.name]);
      return code === 0;
    } catch (error) {
      throw new Error(`Failed to get status: ${error}`);
    }
  }

  async isRunning(): Promise<boolean> {
    return this.show();
  }

  async delete(): Promise<boolean> {
    try {
      const { code } = await this.runner.run(["node", "delete", this.name]);
      return code === 0;
    } catch (error) {
      throw new Error(`Failed to delete node: ${error}`);
    }
  }

  static async create(name: string, runner: Runner = new Runner()): Promise<Node> {
    try {
      const { code } = await runner.run(["node", "create", name]);
      if (code !== 0) {
        throw new Error(`Node creation failed with code: ${code}`);
      }
      return new Node(name, runner);
    } catch (error) {
      throw new Error(`Failed to create node: ${error}`);
    }
  }
}
