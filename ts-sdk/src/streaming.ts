export interface StreamEvent {
  type: string;
  data: unknown;
}

function parseEvent(record: string): StreamEvent | undefined {
  const data: string[] = [];
  let type = "message";

  for (const line of record.split(/\r?\n/)) {
    if (line.startsWith(":")) continue;
    const separator = line.indexOf(":");
    const field = separator === -1 ? line : line.slice(0, separator);
    const value = separator === -1 ? "" : line.slice(separator + 1).replace(/^ /, "");
    if (field === "event") type = value;
    if (field === "data") data.push(value);
  }

  if (data.length === 0) return undefined;
  return { type, data: JSON.parse(data.join("\n")) };
}

export async function* streamEvents(
  response: Response,
): AsyncGenerator<StreamEvent> {
  if (!response.body) {
    throw new Error("SSE response has no body");
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  const emitRecords = function* (): Generator<StreamEvent> {
    let boundary = buffer.search(/\r?\n\r?\n/);
    while (boundary !== -1) {
      const event = parseEvent(buffer.slice(0, boundary));
      buffer = buffer.slice(boundary).replace(/^\r?\n\r?\n/, "");
      if (event) yield event;
      boundary = buffer.search(/\r?\n\r?\n/);
    }
  };

  while (true) {
    const { done, value } = await reader.read();
    buffer += decoder.decode(value, { stream: !done });
    yield* emitRecords();
    if (done) break;
  }

  const finalEvent = parseEvent(buffer);
  if (finalEvent) yield finalEvent;
}
