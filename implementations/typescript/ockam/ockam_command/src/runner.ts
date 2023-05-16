import { spawn } from "child_process";

export interface RunnerOutput {
  code: number | null;
  stdout: string;
  stderr: string;
}

export class Runner {
  static run(command: string, args: string[]) {
    return new Promise<RunnerOutput>((resolve, reject) => {
      let stdout = "";
      let stderr = "";
      const childProcess = spawn(command, args);

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
