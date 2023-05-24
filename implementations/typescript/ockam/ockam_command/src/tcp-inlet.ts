import { Runner, RunnerOutput } from "./runner";

export class TCPInlet {
  static async create(from: string, to: string) {
    const { code } = await Runner.run([
      "tcp-inlet",
      "create",
      "--from",
      from,
      "--to",
      to,
    ]);

    return new Promise<void>((resolve, reject) => {
      code === 0 ? resolve() : reject();
    });
  }
}
