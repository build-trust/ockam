import { Runner, RunnerOutput } from "./runner";

export class Project {
  static async authenticate(ticket: string = "ticket") {
    const { code } = await Runner.run(["project", "authenticate", ticket]);

    return new Promise<void>((resolve, reject) => {
      code === 0 ? resolve() : reject();
    });
  }
}
