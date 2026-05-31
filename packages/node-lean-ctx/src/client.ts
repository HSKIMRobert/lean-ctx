import { execFileSync } from "child_process";

export interface LeanCtxOptions {
  binary?: string;
  projectRoot?: string;
  timeout?: number;
}

export class LeanCtxClient {
  private binary: string;
  private projectRoot: string;
  private timeout: number;

  constructor(options: LeanCtxOptions = {}) {
    this.binary = options.binary ?? "lean-ctx";
    this.projectRoot = options.projectRoot ?? process.cwd();
    this.timeout = options.timeout ?? 30000;
  }

  read(path: string, mode: string = "auto"): string {
    return this.run(["read", path, "--mode", mode]);
  }

  search(pattern: string, path?: string): string {
    const args = ["grep", pattern];
    if (path) args.push(path);
    return this.run(args);
  }

  shell(command: string): string {
    return this.run(["-c", command]);
  }

  gain(): Record<string, unknown> {
    const output = this.run(["gain", "--json"]);
    try {
      return JSON.parse(output);
    } catch {
      return { raw: output };
    }
  }

  benchmark(
    path?: string,
    jsonOutput: boolean = true
  ): Record<string, unknown> {
    const args = ["benchmark", "eval"];
    if (path) args.push(path);
    if (jsonOutput) args.push("--json");
    const output = this.run(args);
    try {
      return JSON.parse(output);
    } catch {
      return { raw: output };
    }
  }

  private run(args: string[]): string {
    try {
      // execFileSync passes argv directly to the binary WITHOUT spawning a
      // shell, so caller-provided values (search patterns, paths, shell
      // commands) can never be interpreted as shell metacharacters. This
      // closes the shell-command-injection class (CodeQL js/shell-command-
      // constructed-from-input). `lean-ctx -c "<cmd>"` still works: the whole
      // command arrives as a single argv element for lean-ctx's own wrapper.
      const result = execFileSync(this.binary, args, {
        cwd: this.projectRoot,
        timeout: this.timeout,
        encoding: "utf-8",
        stdio: ["pipe", "pipe", "pipe"],
      });
      return result.trim();
    } catch (error: unknown) {
      if (
        error instanceof Error &&
        "code" in error &&
        (error as NodeJS.ErrnoException).code === "ENOENT"
      ) {
        throw new Error(
          `lean-ctx binary not found at '${this.binary}'. ` +
            "Install: curl -fsSL https://leanctx.com/install.sh | sh"
        );
      }
      throw error;
    }
  }
}
