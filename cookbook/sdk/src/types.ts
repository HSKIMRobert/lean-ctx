export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

export type JsonObject = { [key: string]: JsonValue };

export type ToolArguments = JsonObject;

export interface ListToolsResponse {
  tools: unknown[];
  total: number;
  offset: number;
  limit: number;
}

export interface ToolCallResponse {
  result: unknown;
}

export type ConsistencyLevel = 'local' | 'eventual' | 'strong';

export interface ContextEventV1 {
  id: number;
  workspaceId: string;
  channelId: string;
  kind: string;
  actor?: string | null;
  timestamp: string;
  version: number;
  parentId: number | null;
  consistencyLevel: ConsistencyLevel;
  payload: JsonValue;
}
