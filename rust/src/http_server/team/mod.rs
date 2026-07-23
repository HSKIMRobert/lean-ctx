use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use anyhow::{Context, Result, anyhow};
use axum::{
    Router,
    body::{self, Body},
    extract::{Extension, Json, Query, State},
    http::{Request, StatusCode, header},
    middleware::{self, Next},
    response::sse::{Event as SseEvent, KeepAlive, Sse},
    response::{IntoResponse, Response},
    routing::get,
};
use futures::Stream;
use md5::{Digest, Md5};
use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequest, CallToolRequestParams, CallToolResult, ClientJsonRpcMessage,
        ClientRequest, JsonRpcRequest, NumberOrString, ServerJsonRpcMessage, ServerResult,
    },
    service::{RequestContext, RoleServer, serve_directly},
    transport::{OneshotTransport, StreamableHttpService},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;
use tokio::time::Duration;

use crate::tools::LeanCtxServer;

mod config;
pub mod connectors;
mod engine;
mod handlers;
mod helpers;
mod request_pipeline;
pub mod roles;
mod server;
mod state;

pub use config::*;
#[allow(clippy::wildcard_imports)]
use engine::*;
#[allow(clippy::wildcard_imports)]
use handlers::*;
pub(crate) use helpers::required_scopes;
use helpers::{hex_lower, parse_sha256_hex, sha256_hex};
#[allow(clippy::wildcard_imports)]
use request_pipeline::*;
pub use roles::TeamRole;
pub use server::*;
pub use state::*;
use state::{EventsQuery, TeamAuthContext, ToolCallBody, ToolsQuery};

const WORKSPACE_ARG_KEY: &str = "workspaceId";
const CHANNEL_ARG_KEY: &str = "channelId";
/// Per-call agent identity (enterprise#28). On the `/v1` REST surface the
/// server overwrites this with `team:<token_id>` — identity is auth-derived,
/// never client-claimed. Raw MCP clients may set it cooperatively (same trust
/// model as local `ctx_agent register`).
const AGENT_ARG_KEY: &str = "agentId";
const WORKSPACE_HEADER: &str = "x-leanctx-workspace";

#[cfg(test)]
mod tests;
