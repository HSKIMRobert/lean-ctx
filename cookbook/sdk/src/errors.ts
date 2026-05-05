import type { JsonValue } from "./types.js";

export class LeanCtxHttpError extends Error {
  readonly status: number;
  readonly method: string;
  readonly url: string;
  readonly errorCode: string | undefined;
  readonly body: JsonValue | string | undefined;

  constructor(opts: {
    status: number;
    method: string;
    url: string;
    message: string;
    errorCode?: string;
    body?: JsonValue | string;
  }) {
    super(opts.message);
    this.name = "LeanCtxHttpError";
    this.status = opts.status;
    this.method = opts.method;
    this.url = opts.url;
    this.errorCode = opts.errorCode;
    this.body = opts.body;
  }
}
