import fs from "fs";
import path from "path";
import { spawn } from "child_process";

export interface RunnerOutput {
  code: number | null;
  stdout: string;
  stderr: string;
}

export class Runner {
  static async run(
    args: string[],
    home: string = path.join(__dirname, "..", "install"),
  ) {
    const binaryPath = path.join(home, "bin", "ockam");
    const command = fs.existsSync(binaryPath) ? binaryPath : "ockam";

    const env = Object.create(process.env);
    env.OCKAM_HOME = home;

    return Runner.exec(command, args, { env: env });
  }

  static async exec(command: string, args: string[], env: Object) {
    return new Promise<RunnerOutput>((resolve, reject) => {
      let stdout = "";
      let stderr = "";
      const childProcess = spawn(command, args, env);

      childProcess.stdout.on("data", (data: Buffer) => {
        stdout += data.toString();
      });

      childProcess.stderr.on("data", (data: Buffer) => {
        stderr += data.toString();
      });

      childProcess.on("close", (code: number | null) => {
        resolve({ code, stdout, stderr });
      });

      childProcess.on("error", (err: Error) => {
        reject(err);
      });
    });
  }
}
