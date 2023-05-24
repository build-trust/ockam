import { Runner, RunnerOutput } from "./runner";

export class Node {
  name: string;

  constructor(name: string) {
    this.name = name;
  }

  async show() {
    const { code } = await Runner.run(["node", "show", this.name]);
    return code === 0;
  }

  async isRunning() {
    return this.show();
  }

  async delete() {
    const { code } = await Runner.run(["node", "delete", this.name]);
    return code === 0;
  }

  static async create(name: string): Promise<Node | null> {
    const { code } = await Runner.run(["node", "create", name]);

    return new Promise<Node | null>((resolve, reject) => {
      code === 0 ? resolve(new Node(name)) : reject();
    });
  }
}
