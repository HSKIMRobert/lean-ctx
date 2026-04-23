#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

const U = 'https://leanctx.com';
const G = 'https://github.com/yvgude/lean-ctx';

// ═══════════════════════════════════════════════════════════════════
// 12 CONTENT CATEGORIES × 30 TWEETS = 360 UNIQUE VIRAL TWEETS
// Dynamic placeholders: {{STARS}}, {{FORKS}}, {{NPM_DL}}, {{CRATES_DL}},
//   {{VERSION}}, {{RELEASES}}, {{DISCORD}}, {{EDITORS}}, {{TOOLS}},
//   {{LANGS}}, {{LOC}}, {{ISSUES}}
// Filled at runtime from live API data
// ═══════════════════════════════════════════════════════════════════

const TWEETS = {

// ── SLOT 0 (06:00 UTC) — MORNING HOOK / BOLD CLAIM ──────────────
morningHook: [
`Your AI agent wastes tokens on every file read.\n\nlean-ctx caches reads: ~13 tokens on re-read vs thousands.\n\n${U}\n\n#AI #DevTools #TokenSavings`,
`Stop paying for tokens your AI doesn't need.\n\n88% average savings per coding session.\n\n${U}\n\n#AI #DevTools #SaveMoney`,
`{{LOC}} lines of Rust. One mission: cut your AI token costs.\n\nlean-ctx compresses everything before it hits the LLM.\n\n${U}\n\n#Rust #AI #DevTools`,
`Most AI coding agents re-read the same files 10+ times per session.\n\nlean-ctx makes re-reads nearly free at ~13 tokens each.\n\n${U}\n\n#AI #CodingAgent`,
`Every file your AI reads without compression wastes money.\n\n{{TOOLS}} MCP tools. 10 read modes. 90+ shell patterns.\n\n${U}\n\n#AI #MCP #DevTools`,
`{{STARS}} developers starred lean-ctx on GitHub.\n\nOpen source. Written in Rust. Zero runtime dependencies.\n\n${U}\n\n#OpenSource #AI #DevTools`,
`Your AI doesn't need 500 lines to understand 3 changes.\n\nctx_read mode=diff shows only what changed.\n\n${U}\n\n#AI #DevTools`,
`{{NPM_DL}} npm downloads this month. Developers are switching.\n\nlean-ctx works with {{EDITORS}} AI editors out of the box.\n\n${U}\n\n#AI #DevTools #npm`,
`A 5MB binary that saves you $30-100/month on AI tokens.\n\nNo runtime dependencies. Installs in seconds.\n\n${U}\n\n#AI #DevTools #Rust`,
`lean-ctx isn't just caching. It's context engineering.\n\nAST analysis. Delta compression. Intent-aware preloading.\n\n${U}\n\n#AI #ContextEngineering`,
`{{RELEASES}} releases. We ship fast and we don't stop.\n\nlean-ctx gets better every single day.\n\n${U}\n\n#AI #DevTools #OpenSource`,
`Git diff output sent to your AI? Compressed 60-90% automatically.\n\nZero config. Just install lean-ctx.\n\n${U}\n\n#AI #DevTools #Git`,
`Your $20/month AI subscription burns tokens on every read.\n\nlean-ctx cuts that burn rate by 88%.\n\n${U}\n\n#AI #DevTools`,
`One tool. {{TOOLS}} MCP endpoints. Every major AI editor supported.\n\nlean-ctx is the context layer your agent needs.\n\n${U}\n\n#MCP #AI #DevTools`,
`AI coding costs scale with tokens. lean-ctx scales them down.\n\n88% reduction. Real benchmark data.\n\n${U}/benchmark\n\n#AI #DevTools`,
`Reading a 1000-line file costs ~4000 tokens.\n\nlean-ctx signature mode: ~200 tokens. 95% saved.\n\n${U}\n\n#AI #TokenSavings`,
`The problem isn't your AI model. It's the context you feed it.\n\nlean-ctx engineers optimal context automatically.\n\n${U}\n\n#AI #ContextEngineering`,
`Rust for speed. MCP for compatibility. lean-ctx for savings.\n\nWorks with Cursor, Claude Code, Copilot, and 18 more.\n\n${U}\n\n#AI #Rust #MCP`,
`Your AI re-reads package.json 15 times per session.\n\nWith lean-ctx, reads 2-15 cost ~13 tokens each.\n\n${U}\n\n#AI #DevTools`,
`{{FORKS}} forks on GitHub. The community is building on lean-ctx.\n\nJoin {{DISCORD}} developers on Discord.\n\n${U}\n\n#OpenSource #AI`,
`Without context compression, AI agents hit token limits fast.\n\nlean-ctx keeps you in the zone longer.\n\n${U}\n\n#AI #DevTools`,
`Tree-sitter AST analysis for {{LANGS}} programming languages.\n\nlean-ctx understands code structure, not just text.\n\n${U}\n\n#AI #TreeSitter #DevTools`,
`Your AI explores your codebase blind.\n\nctx_overview gives it a task-relevant map in one call.\n\n${U}\n\n#AI #DevTools`,
`Token costs add up: $50, $100, $200/month.\n\nlean-ctx cuts 88% of that. Every single month.\n\n${U}\n\n#AI #SaveMoney #DevTools`,
`lean-ctx: the invisible layer between your code and your AI.\n\nEverything compressed. Nothing lost.\n\n${U}\n\n#AI #DevTools #MCP`,
`6 contributors. {{LOC}} lines of Rust. 100% open source.\n\nBuilt for developers, by developers.\n\n${U}\n\n#OpenSource #Rust #AI`,
`Copilot, Claude Code, Cursor — they all burn tokens.\n\nlean-ctx compresses context for all of them.\n\n${U}\n\n#AI #Copilot #ClaudeCode`,
`Why send 10,000 tokens when 1,200 carry the same meaning?\n\nlean-ctx: 88% compression. Zero information loss.\n\n${U}\n\n#AI #Compression`,
`Every AI coding session starts with exploration.\n\nlean-ctx makes exploration nearly token-free.\n\n${U}\n\n#AI #DevTools`,
`The most expensive line of code is the one your AI reads twice.\n\nlean-ctx makes re-reads cost ~13 tokens.\n\n${U}\n\n#AI #DevTools`,
],

// ── SLOT 1 (07:30 UTC) — FEATURE SPOTLIGHT ───────────────────────
featureSpotlight: [
`ctx_read: 10 different modes to read files.\n\nfull, map, signatures, diff, aggressive, entropy, task, reference, auto, lines:N-M.\n\n${U}/docs/tools/\n\n#AI #MCP`,
`ctx_shell: compressed shell output for 90+ command patterns.\n\ngit, npm, cargo, docker, kubectl — all optimized.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`ctx_overview: one call gives your AI agent a complete project map.\n\nTask-relevant. No blind exploration needed.\n\n${U}/docs/tools/\n\n#AI #MCP`,
`ctx_search: compressed grep/ripgrep results.\n\nSame results, 50-80% fewer tokens sent to the LLM.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`ctx_tree: directory listing with intelligent depth control.\n\nYour AI sees project structure without token waste.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`ctx_session: persistent task context across chat sessions.\n\nNo more re-explaining what you were working on.\n\n${U}/docs/tools/\n\n#AI #MCP`,
`ctx_agent: multi-agent coordination built in.\n\nRegister agents. Share context. Handoff tasks.\n\n${U}/docs/tools/\n\n#AI #MultiAgent #MCP`,
`ctx_preload: proactively cache task-relevant files.\n\nWhen the AI reads them later, each costs ~13 tokens.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`ctx_dedup: find shared imports and boilerplate across files.\n\nEliminates redundant context automatically.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`ctx_compress: squeeze any text to its semantic minimum.\n\nKeeps meaning. Removes noise. Saves tokens.\n\n${U}/docs/tools/\n\n#AI #MCP`,
`ctx_graph: build a project intelligence graph.\n\nDependency analysis + related file discovery.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`ctx_heatmap: see which files your AI reads most.\n\nOptimize your workflow based on real access data.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`ctx_fill: maximize information within a token budget.\n\nGive it a limit — it prioritizes the most relevant context.\n\n${U}/docs/tools/\n\n#AI #MCP`,
`ctx_architecture: auto-generated architectural overview.\n\nOnboard your AI agent to any project in seconds.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`ctx_semantic_search: meaning-based code search.\n\nFind code by what it does, not just what it says.\n\n${U}/docs/tools/\n\n#AI #MCP`,
`ctx_knowledge: persistent memory across sessions.\n\nRemember, recall, search, timeline — real agent memory.\n\n${U}/docs/tools/\n\n#AI #MCP`,
`ctx_edit: file editing with automatic re-read.\n\nEdit and confirm in one step. No manual refresh.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`Shell hook: transparent compression for 90+ commands.\n\ngit status, npm list, docker ps — all compressed.\n\n${U}/docs/shell-hook/\n\n#AI #DevTools`,
`10 read modes in ctx_read. Each optimized for a different task.\n\n"signatures" for API surface. "diff" for changes. "entropy" for unique content.\n\n${U}\n\n#AI #MCP`,
`ctx_read mode=aggressive: maximum compression.\n\nKeeps only the essential code structure and logic.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`ctx_read mode=map: file overview without reading everything.\n\nFunction names, class structure, key exports.\n\n${U}/docs/tools/\n\n#AI #MCP`,
`ctx_read mode=task: context filtered by your current task.\n\nOnly shows code relevant to what you're working on.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`ctx_read mode=entropy: information-theory based compression.\n\nKeeps high-entropy (unique) content, removes boilerplate.\n\n${U}/docs/tools/\n\n#AI #MCP`,
`ctx_read mode=reference: extract references and dependencies.\n\nSee what a file imports, exports, and depends on.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`Ledger system: tracks every file read, its cost, and compression ratio.\n\nlean-ctx knows exactly what your AI has seen.\n\n${U}/docs/concepts/\n\n#AI #MCP`,
`Intent engine: predicts what your AI needs next.\n\nProactive caching based on access patterns.\n\n${U}/docs/concepts/\n\n#AI #DevTools`,
`Delta mode: only sends changed hunks since last read.\n\n3 lines changed in 500-line file? Your AI sees only those 3.\n\n${U}/docs/tools/\n\n#AI #MCP`,
`Function references: F1, F2, F3... persist across calls.\n\nRe-read a function for ~13 tokens using its reference.\n\n${U}/docs/tools/\n\n#AI #DevTools`,
`ctx_diary: structured agent diary for discoveries and decisions.\n\nProgress, blockers, insights — all tracked.\n\n${U}/docs/tools/\n\n#AI #MCP`,
`Auto checkpoint: lean-ctx saves context state every 15 calls.\n\nCrash recovery built in. Zero data loss.\n\n${U}/docs/concepts/\n\n#AI #DevTools`,
],

// ── SLOT 2 (09:00 UTC) — PRO TIP / HOW-TO ───────────────────────
proTip: [
`Pro tip: Start every AI session with ctx_overview.\n\nGives your agent a task-relevant project map. Saves thousands of tokens.\n\n${U}\n\n#AI #DevTools #ProTip`,
`Pro tip: Use ctx_read mode=signatures for large files.\n\nYour AI sees the API surface without reading every line.\n\n${U}\n\n#AI #ProTip`,
`Pro tip: Run "lean-ctx gain" to see your real token savings.\n\nExact numbers. Session by session. Dollar amounts.\n\n${U}\n\n#AI #DevTools #ProTip`,
`Pro tip: ctx_preload at session start caches task-relevant files.\n\nEvery subsequent read costs ~13 tokens.\n\n${U}\n\n#AI #ProTip`,
`Pro tip: Disable lean-ctx for a single command:\n\nLEAN_CTX_DISABLED=1 your-command\n\nUseful for auth flows or binary output.\n\n${U}\n\n#DevTools #ProTip`,
`Pro tip: ctx_read mode=diff after editing shows only your changes.\n\nPerfect for code review workflows.\n\n${U}\n\n#AI #ProTip`,
`Pro tip: Use ctx_fill with a token budget.\n\nIt packs maximum information into your limit.\n\n${U}\n\n#AI #DevTools #ProTip`,
`Pro tip: ctx_search replaces grep with compressed results.\n\nSame patterns. 50-80% fewer tokens.\n\n${U}\n\n#AI #ProTip`,
`Pro tip: Set up lean-ctx for all your editors at once:\n\nlean-ctx setup\n\nAuto-detects {{EDITORS}} supported editors.\n\n${U}\n\n#DevTools #ProTip`,
`Pro tip: ctx_architecture at the start of a new project.\n\nOnboards your AI agent in seconds, not minutes.\n\n${U}\n\n#AI #ProTip`,
`Pro tip: ctx_agent with action=handoff for multi-agent workflows.\n\nPass context between agents cleanly.\n\n${U}\n\n#AI #MultiAgent #ProTip`,
`Pro tip: ctx_tree with controlled depth to avoid overwhelming output.\n\nShow 2 levels deep, not the whole tree.\n\n${U}\n\n#AI #DevTools #ProTip`,
`Pro tip: Update lean-ctx with a single command:\n\nlean-ctx update\n\nAlways get the latest optimizations.\n\n${U}\n\n#DevTools #ProTip`,
`Pro tip: Use ctx_read mode=lines:10-50 for surgical reads.\n\nOnly the lines you need. Nothing more.\n\n${U}\n\n#AI #ProTip`,
`Pro tip: ctx_session saves your current task context.\n\nClose the chat. Come back tomorrow. Context preserved.\n\n${U}\n\n#AI #DevTools #ProTip`,
`Pro tip: ctx_knowledge with action=remember stores insights.\n\nYour AI agent builds persistent project knowledge.\n\n${U}\n\n#AI #ProTip`,
`Pro tip: Use ctx_heatmap to find your most-read files.\n\nOptimize those files for better compression.\n\n${U}\n\n#AI #DevTools #ProTip`,
`Pro tip: ctx_dedup before large refactors.\n\nFinds duplicated code across files your AI would read twice.\n\n${U}\n\n#AI #ProTip`,
`Pro tip: Install via brew for automatic updates:\n\nbrew install lean-ctx\n\nStay current. Zero effort.\n\n${U}\n\n#DevTools #macOS #ProTip`,
`Pro tip: ctx_compress on clipboard content before pasting.\n\nCompress docs, logs, or errors before feeding to your AI.\n\n${U}\n\n#AI #ProTip`,
`Pro tip: Check compression ratios with ctx_read mode=full + fresh=true.\n\nSee exactly how much lean-ctx saves on each file.\n\n${U}\n\n#AI #DevTools #ProTip`,
`Pro tip: Use ctx_graph to discover hidden dependencies.\n\nYour AI agent sees the full picture before making changes.\n\n${U}\n\n#AI #ProTip`,
`Pro tip: ctx_read with fresh=true forces a re-read.\n\nUseful after external file changes outside your editor.\n\n${U}\n\n#AI #DevTools #ProTip`,
`Pro tip: lean-ctx works instantly with npx:\n\nnpx lean-ctx-bin setup\n\nNo global install needed. Try it risk-free.\n\n${U}\n\n#DevTools #ProTip`,
`Pro tip: ctx_semantic_search for meaning-based code discovery.\n\n"Where do we handle auth?" instead of grepping for keywords.\n\n${U}\n\n#AI #ProTip`,
`Pro tip: ctx_diary with category=blocker tracks what's stuck.\n\nYour AI agent remembers blockers across sessions.\n\n${U}\n\n#AI #DevTools #ProTip`,
`Pro tip: Use multiple read modes in one session.\n\nStart with signatures, drill into specific sections with lines:N-M.\n\n${U}\n\n#AI #ProTip`,
`Pro tip: ctx_agent action=sync for real-time multi-agent state.\n\nAll agents see the same context instantly.\n\n${U}\n\n#AI #MultiAgent #ProTip`,
`Pro tip: Set CEP compliance mode for measurable efficiency:\n\nScoring from 0-100 for every AI interaction.\n\n${U}/protocols/cep\n\n#AI #DevTools #ProTip`,
`Pro tip: CCP protocol for cross-session memory.\n\nYour AI remembers discoveries, decisions, and progress.\n\n${U}/protocols/ccp\n\n#AI #DevTools #ProTip`,
],

// ── SLOT 3 (10:30 UTC) — STATS WITH REAL NUMBERS ─────────────────
statsNumbers: [
`{{STARS}} GitHub stars.\n{{FORKS}} forks.\n{{RELEASES}} releases.\n\nlean-ctx is growing fast.\n\n${U}\n\n#AI #DevTools #OpenSource`,
`{{NPM_DL}} npm downloads this month alone.\n\nDevelopers are choosing lean-ctx for token savings.\n\n${U}\n\n#AI #DevTools #npm`,
`88% average token savings per coding session.\n\nReal benchmark data. Real projects. Real savings.\n\n${U}/benchmark\n\n#AI #DevTools`,
`{{TOOLS}} MCP tools. 10 read modes. 90+ shell patterns.\n\nAll in a single 5MB Rust binary.\n\n${U}\n\n#AI #MCP #Rust`,
`{{LOC}} lines of Rust code.\n\nEvery line optimized for one goal: compress your AI context.\n\n${U}\n\n#Rust #AI #DevTools`,
`{{EDITORS}} AI editors supported out of the box.\n\nCursor, Claude Code, Copilot, Windsurf, Zed, Codex, Gemini CLI, and more.\n\n${U}\n\n#AI #DevTools`,
`{{RELEASES}} releases since launch.\n\nThat's multiple improvements every single day.\n\n${U}\n\n#AI #DevTools #OpenSource`,
`{{DISCORD}} developers in the Discord community.\n\nReal-time support. Feature requests. Bug reports.\n\n${U}\n\n#AI #DevTools #Discord`,
`{{CRATES_DL}} downloads on crates.io.\n\nRust developers love lean-ctx.\n\n${U}\n\n#Rust #AI #DevTools`,
`6 contributors building lean-ctx together.\n\nOpen source. Community driven. Apache 2.0.\n\n${U}\n\n#OpenSource #AI`,
`Re-read cost: ~13 tokens.\nFirst read of 1000 lines: ~4000 tokens.\nSavings: 99.7%.\n\n${U}\n\n#AI #TokenSavings`,
`{{FORKS}} forks on GitHub.\n\nDevelopers aren't just using lean-ctx — they're building on it.\n\n${U}\n\n#OpenSource #AI #DevTools`,
`Tree-sitter support for {{LANGS}} languages.\n\nPython, TypeScript, Rust, Go, Java, C++, and more.\n\n${U}\n\n#AI #TreeSitter`,
`{{ISSUES}} GitHub issues resolved.\n\nWe listen. We fix. We ship.\n\n${U}\n\n#AI #DevTools #OpenSource`,
`Version {{VERSION}} — and counting.\n\nRapid iteration. Constant improvement.\n\n${U}\n\n#AI #DevTools`,
`Shell hook compresses 90+ command patterns.\n\ngit, npm, cargo, docker, kubectl, ls, find, curl — all covered.\n\n${U}\n\n#AI #DevTools`,
`5MB binary. Zero dependencies. 88% savings.\n\nThat's the lean-ctx value proposition.\n\n${U}\n\n#AI #Rust #DevTools`,
`$30-100+ saved per month on AI token costs.\n\nThat's $360-1200 per year. Per developer.\n\n${U}\n\n#AI #SaveMoney`,
`{{NPM_DL}} npm + {{CRATES_DL}} crates.io downloads.\n\nGrowing every week.\n\n${U}\n\n#AI #DevTools #OpenSource`,
`lean-ctx launched March 23, 2026.\n\n{{STARS}} stars — organic growth only.\n\n${U}\n\n#AI #DevTools`,
`Signature mode: ~200 tokens for a 1000-line file.\n\nFull read: ~4000 tokens.\n\n95% savings on a single read.\n\n${U}\n\n#AI #TokenSavings`,
`git status compressed: 60-90% fewer tokens.\ngit diff compressed: 60-90% fewer tokens.\ngit log compressed: 60-90% fewer tokens.\n\n${U}\n\n#AI #Git`,
`0 telemetry. 0 data sent. 100% local.\n\nYour code never leaves your machine.\n\n${U}\n\n#Privacy #AI #OpenSource`,
`Average session without lean-ctx: 89,800 tokens.\nWith lean-ctx: 10,620 tokens.\n\n88% savings.\n\n${U}/benchmark\n\n#AI #DevTools`,
`lean-ctx is Apache 2.0 licensed.\n\nUse it commercially. Fork it. Build on it.\n\n${G}\n\n#OpenSource #AI`,
`15 topics on GitHub: agentic-coding, ai, context-engineering, cursor, mcp, rust, token-optimization...\n\n${U}\n\n#AI #DevTools`,
`Every file cached. Every re-read costs ~13 tokens.\n\nThat's 99.7% savings on subsequent reads.\n\n${U}\n\n#AI #TokenSavings`,
`One cargo install. One npm install. One brew install.\n\nThree ecosystems. One tool.\n\n${U}\n\n#Rust #npm #DevTools`,
`lean-ctx has been downloaded from 140+ countries.\n\nGlobal adoption. Local execution.\n\n${U}\n\n#AI #DevTools #OpenSource`,
`{{TOOLS}} granular MCP tools + 5 unified tools.\n\nPick your level of control.\n\n${U}/mcp-server\n\n#AI #MCP #DevTools`,
],

// ── SLOT 4 (12:00 UTC) — WITH vs WITHOUT COMPARISON ─────────────
comparison: [
`Without lean-ctx: 89,800 tokens/session.\nWith lean-ctx: 10,620 tokens/session.\n\nThat's 88% savings.\n\n${U}/benchmark\n\n#AI #DevTools`,
`Reading a file:\nWithout: ~4,000 tokens\nWith lean-ctx: ~200 tokens (signatures mode)\n\n95% saved.\n\n${U}\n\n#AI #TokenSavings`,
`Re-reading a file:\nWithout: ~4,000 tokens again\nWith lean-ctx: ~13 tokens\n\n99.7% saved.\n\n${U}\n\n#AI #DevTools`,
`git status output:\nWithout: 200-500 tokens\nWith lean-ctx: 40-80 tokens\n\n60-84% compressed.\n\n${U}\n\n#AI #Git`,
`Exploring a project:\nWithout: thousands of tokens\nWith ctx_overview: one call, task-relevant map\n\n${U}\n\n#AI #DevTools`,
`Multi-file refactor:\nWithout: re-read everything\nWith lean-ctx: delta mode shows only changes\n\n${U}\n\n#AI #DevTools`,
`Code review:\nWithout: read entire files\nWith ctx_read diff mode: see only what changed\n\n${U}\n\n#AI #CodeReview`,
`New project onboarding:\nWithout: agent explores blindly for 5 min\nWith ctx_architecture: instant overview\n\n${U}\n\n#AI #DevTools`,
`npm list output:\nWithout: 1000+ tokens of dependency tree\nWith lean-ctx shell hook: 100-200 tokens\n\n${U}\n\n#AI #npm`,
`Monthly AI costs:\nWithout lean-ctx: $100+\nWith lean-ctx: $12-20\n\n$80+ saved every month.\n\n${U}\n\n#AI #SaveMoney`,
`Session start:\nWithout: agent reads 20 files (80K tokens)\nWith ctx_preload: cached reads cost ~260 tokens total\n\n${U}\n\n#AI #DevTools`,
`Finding code:\nWithout: grep returns full lines\nWith ctx_search: compressed results, 50-80% fewer tokens\n\n${U}\n\n#AI #DevTools`,
`Cross-session context:\nWithout: re-explain everything\nWith ctx_session: context persists automatically\n\n${U}\n\n#AI #DevTools`,
`docker ps output:\nWithout: 300-500 tokens\nWith lean-ctx: 50-100 tokens\n\n80% compressed.\n\n${U}\n\n#AI #Docker`,
`Directory listing:\nWithout: ls -la dumps everything\nWith ctx_tree: structured, depth-controlled\n\n${U}\n\n#AI #DevTools`,
`Agent exploration:\nWithout: 5-10 tool calls to understand project\nWith ctx_overview: 1 call, full context\n\n${U}\n\n#AI #DevTools`,
`Token budget per month:\nWithout lean-ctx: 2.7M tokens\nWith lean-ctx: 320K tokens\n\n88% reduction.\n\n${U}/benchmark\n\n#AI #DevTools`,
`File change detection:\nWithout: re-read entire file\nWith ctx_read diff: only the changed hunks\n\n${U}\n\n#AI #DevTools`,
`Multi-agent workflow:\nWithout: each agent reads independently\nWith ctx_agent: shared cache, zero duplication\n\n${U}\n\n#AI #MultiAgent`,
`Large codebase:\nWithout: token limit hit in 5 min\nWith lean-ctx: work all day within budget\n\n${U}\n\n#AI #DevTools`,
`Understanding dependencies:\nWithout: manually trace imports\nWith ctx_graph: automatic dependency analysis\n\n${U}\n\n#AI #DevTools`,
`API surface review:\nWithout: read 2000 lines\nWith ctx_read signatures: ~100 tokens for the full API\n\n${U}\n\n#AI #DevTools`,
`Error debugging:\nWithout: paste 500 lines of logs\nWith ctx_compress: extract only the relevant error\n\n${U}\n\n#AI #DevTools`,
`Boilerplate detection:\nWithout: agent reads same patterns repeatedly\nWith ctx_dedup: eliminated automatically\n\n${U}\n\n#AI #DevTools`,
`git log review:\nWithout: 1000+ tokens of commit history\nWith lean-ctx: 100-200 tokens, same info\n\n${U}\n\n#AI #Git`,
`kubectl output:\nWithout: 500+ tokens of pod/service data\nWith lean-ctx: 80-150 tokens compressed\n\n${U}\n\n#AI #Kubernetes`,
`File search:\nWithout: find returns every match fully\nWith ctx_search: compressed, deduplicated results\n\n${U}\n\n#AI #DevTools`,
`Second coding session:\nWithout: cold start, explain everything again\nWith CCP protocol: agent remembers yesterday\n\n${U}/protocols/ccp\n\n#AI`,
`Pair programming with AI:\nWithout: AI forgets context every message\nWith lean-ctx: persistent, compressed context\n\n${U}\n\n#AI #DevTools`,
`Annual token cost:\nWithout: $1,200+\nWith lean-ctx: $144-240\n\nSave $960+/year.\n\n${U}\n\n#AI #SaveMoney`,
],

// ── SLOT 5 (13:30 UTC) — EDITOR SPOTLIGHT ────────────────────────
editorSpotlight: [
`Using Cursor? lean-ctx integrates automatically.\n\nlean-ctx setup auto-detects your Cursor config and installs.\n\n${U}\n\n#Cursor #AI #DevTools`,
`Claude Code + lean-ctx = 88% fewer tokens per session.\n\nSame AI power. Fraction of the cost.\n\n${U}\n\n#ClaudeCode #AI #DevTools`,
`GitHub Copilot users: lean-ctx compresses your context.\n\nFaster responses. Lower costs. Same quality.\n\n${U}\n\n#Copilot #AI #DevTools`,
`Windsurf + lean-ctx: context-aware coding on a budget.\n\nAutomatic setup. Zero configuration.\n\n${U}\n\n#Windsurf #AI #DevTools`,
`Zed editor supported. lean-ctx setup handles everything.\n\nFast editor + fast context = best workflow.\n\n${U}\n\n#Zed #AI #DevTools`,
`OpenAI Codex CLI + lean-ctx: compressed context for every call.\n\nGet more from every token.\n\n${U}\n\n#Codex #OpenAI #AI`,
`Gemini CLI users: lean-ctx works out of the box.\n\nGoogle AI + context compression = cost-effective coding.\n\n${U}\n\n#Gemini #AI #DevTools`,
`VS Code + Copilot + lean-ctx = the ultimate setup.\n\nCompress context before it reaches the model.\n\n${U}\n\n#VSCode #Copilot #AI`,
`Amazon Q Developer supported.\n\nlean-ctx compresses context for AWS AI coding too.\n\n${U}\n\n#AWS #AI #DevTools`,
`Aider + lean-ctx: terminal-based AI coding with 88% savings.\n\nPure CLI workflow. Maximum efficiency.\n\n${U}\n\n#Aider #AI #DevTools`,
`Cline users: lean-ctx reduces token consumption dramatically.\n\nMore coding, less token budget anxiety.\n\n${U}\n\n#Cline #AI #DevTools`,
`Continue.dev + lean-ctx: open-source AI coding, optimized.\n\nBoth open source. Both built for developers.\n\n${U}\n\n#ContinueDev #OpenSource #AI`,
`Roo Code supported by lean-ctx.\n\n{{EDITORS}} editors. One installation command.\n\n${U}\n\n#AI #DevTools`,
`Augment Code + lean-ctx: enterprise AI coding, compressed.\n\nScale your AI usage without scaling costs.\n\n${U}\n\n#AI #DevTools #Enterprise`,
`JetBrains AI + lean-ctx: IntelliJ, PyCharm, WebStorm — all supported.\n\nContext compression for the JetBrains ecosystem.\n\n${U}\n\n#JetBrains #AI`,
`Neovim + lean-ctx: for developers who live in the terminal.\n\nMinimal setup. Maximum savings.\n\n${U}\n\n#Neovim #AI #DevTools`,
`Trae IDE supported. lean-ctx works with ByteDance's AI editor.\n\nGlobal editor support.\n\n${U}\n\n#AI #DevTools`,
`PearAI + lean-ctx: compressed context for AI pair programming.\n\nCollaborate with AI efficiently.\n\n${U}\n\n#AI #DevTools`,
`lean-ctx setup detects all {{EDITORS}} supported editors automatically.\n\nOne command. Every editor configured.\n\n${U}\n\n#AI #DevTools`,
`Switching AI editors? lean-ctx works with all of them.\n\nNo lock-in. No reconfiguration.\n\n${U}\n\n#AI #DevTools`,
`Claude Desktop + lean-ctx MCP server.\n\nFull {{TOOLS}}-tool integration for Anthropic's client.\n\n${U}/mcp-server\n\n#Claude #AI #MCP`,
`Copilot Chat in VS Code burns tokens fast.\n\nlean-ctx compresses every file read automatically.\n\n${U}\n\n#Copilot #VSCode #AI`,
`Cursor Composer + lean-ctx: multi-file edits, compressed context.\n\nEdit across files without hitting token limits.\n\n${U}\n\n#Cursor #AI`,
`Claude Code in the terminal? lean-ctx is the perfect companion.\n\nCompressed reads. Cached re-reads. Shell compression.\n\n${U}\n\n#ClaudeCode #AI`,
`Codex in the cloud? lean-ctx still helps.\n\nContext compression works regardless of where the model runs.\n\n${U}\n\n#OpenAI #Codex #AI`,
`Using multiple AI editors? lean-ctx is editor-agnostic.\n\nSame compression layer for all your tools.\n\n${U}\n\n#AI #DevTools`,
`Void editor supported by lean-ctx.\n\nOpen-source editors deserve open-source tools.\n\n${U}\n\n#OpenSource #AI`,
`Kilo Code + lean-ctx: budget-friendly AI coding.\n\nCompress context to get the most from every dollar.\n\n${U}\n\n#AI #DevTools`,
`Melty + lean-ctx: AI pair programming with 88% token savings.\n\nFocus on code, not costs.\n\n${U}\n\n#AI #DevTools`,
`{{EDITORS}} editors. Zero config. One command: lean-ctx setup.\n\nIf you use an AI editor, lean-ctx supports it.\n\n${U}\n\n#AI #DevTools`,
],

// ── SLOT 6 (15:00 UTC) — USE CASE / DEVELOPER STORY ─────────────
useCase: [
`Refactoring a 500-file monorepo?\n\nlean-ctx keeps your AI focused with ctx_architecture + delta reads.\n\nNo token budget overflow.\n\n${U}\n\n#AI #DevTools`,
`Debugging a production issue at 2 AM?\n\nctx_compress squeezes error logs before feeding them to your AI.\n\n${U}\n\n#AI #DevTools`,
`Onboarding to a new codebase?\n\nctx_overview + ctx_architecture = instant project understanding.\n\n${U}\n\n#AI #DevTools`,
`Writing tests for legacy code?\n\nctx_read mode=signatures shows the API without reading implementation.\n\n${U}\n\n#AI #Testing`,
`Multi-file migration (JS to TS)?\n\nlean-ctx caches all files. Re-reads during migration cost ~13 tokens each.\n\n${U}\n\n#AI #TypeScript`,
`Code review with AI assistance?\n\nctx_read mode=diff shows only what changed. No noise.\n\n${U}\n\n#AI #CodeReview`,
`Building a new feature across 10 files?\n\nctx_preload caches them all. Your AI works within budget.\n\n${U}\n\n#AI #DevTools`,
`Fixing flaky CI tests?\n\nctx_shell compresses test output. Your AI gets the failures, not the noise.\n\n${U}\n\n#AI #CI #DevTools`,
`Documenting an undocumented API?\n\nctx_read signatures + ctx_graph maps the full API surface.\n\n${U}\n\n#AI #Documentation`,
`Upgrading dependencies?\n\nctx_shell compresses npm/cargo output. See what changed without token waste.\n\n${U}\n\n#AI #DevTools`,
`Working on a Rust project? lean-ctx is written in Rust too.\n\n{{LOC}} lines of Rust. We know the ecosystem.\n\n${U}\n\n#Rust #AI #DevTools`,
`Kubernetes deployment debugging?\n\nlean-ctx shell hook compresses kubectl output by 80%.\n\n${U}\n\n#Kubernetes #AI #DevOps`,
`Microservices architecture?\n\nctx_graph maps cross-service dependencies automatically.\n\n${U}\n\n#AI #Microservices`,
`Long AI coding session (2+ hours)?\n\nlean-ctx ctx_session preserves context so nothing is lost.\n\n${U}\n\n#AI #DevTools`,
`Open source contributor?\n\nctx_overview + ctx_architecture = understand any repo in minutes.\n\n${U}\n\n#OpenSource #AI`,
`Working with Docker?\n\nlean-ctx shell hook compresses docker ps, logs, inspect output.\n\n${U}\n\n#Docker #AI #DevTools`,
`Database schema review?\n\nctx_read mode=signatures extracts table structures. No full dump needed.\n\n${U}\n\n#AI #Database`,
`Security audit with AI?\n\nctx_search finds patterns across the codebase. Compressed results.\n\n${U}\n\n#AI #Security`,
`Prototyping a new idea?\n\nlean-ctx lets your AI explore and iterate without burning through tokens.\n\n${U}\n\n#AI #DevTools`,
`Monorepo with 100+ packages?\n\nctx_tree + ctx_overview: navigate the maze without token waste.\n\n${U}\n\n#AI #Monorepo`,
`Pair programming with multiple AI agents?\n\nctx_agent coordinates context sharing between them.\n\n${U}\n\n#AI #MultiAgent`,
`Setting up a new project from scratch?\n\nlean-ctx compresses scaffold output, config generation, and deps.\n\n${U}\n\n#AI #DevTools`,
`Performance optimization?\n\nctx_heatmap shows which files your AI accesses most. Optimize there first.\n\n${U}\n\n#AI #Performance`,
`Working across frontend and backend?\n\nlean-ctx compresses context for both. No switching needed.\n\n${U}\n\n#AI #FullStack`,
`Writing a CLI tool?\n\nlean-ctx itself is a CLI tool. Built in Rust. 5MB binary.\n\n${U}\n\n#CLI #Rust #AI`,
`Resolving merge conflicts with AI?\n\nctx_read diff mode shows the conflicts. Nothing else.\n\n${U}\n\n#AI #Git #DevTools`,
`Maintaining multiple projects?\n\nctx_session keeps context separate for each project.\n\n${U}\n\n#AI #DevTools`,
`Converting Python 2 to Python 3?\n\nlean-ctx caches every file. Migration reads cost ~13 tokens each.\n\n${U}\n\n#Python #AI`,
`Analyzing complex log files?\n\nctx_compress extracts the signal from the noise. 90% compression.\n\n${U}\n\n#AI #DevOps`,
`Building an API?\n\nctx_read signatures shows endpoint definitions. ctx_graph maps routes.\n\n${U}\n\n#AI #API #DevTools`,
],

// ── SLOT 7 (16:30 UTC) — TECHNICAL DEEP DIVE ────────────────────
techDeepDive: [
`Tree-sitter AST analysis: lean-ctx parses your code structure.\n\n{{LANGS}} languages. Real abstract syntax trees.\n\n${U}\n\n#TreeSitter #AST #AI`,
`Written in Rust: memory-safe, fast, zero-cost abstractions.\n\n{{LOC}} lines. No garbage collector. No runtime.\n\n${U}\n\n#Rust #AI #DevTools`,
`MCP (Model Context Protocol) native.\n\n{{TOOLS}} tools speak the standard protocol for AI tool integration.\n\n${U}/mcp-server\n\n#MCP #AI`,
`Delta compression: only changed hunks are sent.\n\nIf 3 lines change in a 500-line file, your AI sees 3 lines.\n\n${U}\n\n#AI #Compression`,
`Entropy-based read mode: information theory applied to code.\n\nHigh-entropy = unique content kept. Low-entropy = boilerplate removed.\n\n${U}\n\n#AI #InfoTheory`,
`Shell hook architecture: transparent proxy for 90+ commands.\n\nIntercepts, compresses, returns. Zero workflow changes.\n\n${U}\n\n#AI #Shell`,
`Ledger system tracks every file access.\n\nRead count, token cost, compression ratio — all logged.\n\n${U}\n\n#AI #DevTools`,
`Intent engine: ML-based prediction of next file access.\n\nProactive caching reduces latency and tokens.\n\n${U}\n\n#AI #ML #DevTools`,
`Function references (F1, F2, F3...): persistent handles.\n\nRefer to code by reference. Re-read for ~13 tokens.\n\n${U}\n\n#AI #MCP`,
`Single binary architecture: no Python, no Node.js, no Java.\n\nJust a 5MB Rust binary. Copy it anywhere.\n\n${U}\n\n#Rust #AI`,
`Context Continuity Protocol (CCP): cross-session memory.\n\nStandardized format for AI agent memory persistence.\n\n${U}/protocols/ccp\n\n#AI #Protocol`,
`Cognitive Efficiency Protocol (CEP): measurable AI efficiency.\n\nCompliance scoring 0-100. Benchmark your AI workflow.\n\n${U}/protocols/cep\n\n#AI #Protocol`,
`Tree-sitter languages: Python, TypeScript, JavaScript, Rust, Go, Java, C, C++, Ruby, PHP, Swift, Kotlin, and more.\n\n${U}\n\n#AI`,
`Signature extraction: parse function signatures, class definitions, exports.\n\nAPI surface in ~200 tokens for any file.\n\n${U}\n\n#AI #AST`,
`Smart caching: content-addressable storage.\n\nSame content = same cache entry. Hash-based deduplication.\n\n${U}\n\n#AI #DevTools`,
`Multi-agent architecture: register, sync, handoff, diary.\n\nBuilt for the agentic coding future.\n\n${U}\n\n#AI #MultiAgent`,
`Graph analysis: dependency resolution + cycle detection.\n\nctx_graph builds a real project intelligence graph.\n\n${U}\n\n#AI #GraphTheory`,
`Preload engine: predict and cache files before they're needed.\n\nBased on task description and access patterns.\n\n${U}\n\n#AI #DevTools`,
`Compression pipeline: tokenize -> analyze -> compress -> cache.\n\nFour stages. Sub-millisecond latency.\n\n${U}\n\n#AI #Performance`,
`Zero telemetry architecture: no phone-home, no analytics.\n\nAll data stays on your machine. Verify it yourself.\n\n${G}\n\n#Privacy #OpenSource`,
`Auto-checkpoint every 15 tool calls.\n\nContext state saved to disk. Crash recovery built in.\n\n${U}\n\n#AI #Reliability`,
`Build targets: x86_64 and ARM64 for Linux, macOS, Windows.\n\nCross-compiled. Native performance everywhere.\n\n${U}\n\n#Rust #CrossPlatform`,
`Apache 2.0 license: permissive, commercial-friendly.\n\nUse it in your startup. Fork it. Modify it.\n\n${G}\n\n#OpenSource`,
`Semantic search: embedding-based code search.\n\nFind code by meaning, not text matching.\n\n${U}\n\n#AI #SemanticSearch`,
`Knowledge graph: remember, recall, search, timeline, rooms.\n\n5 operations for persistent AI memory.\n\n${U}\n\n#AI #KnowledgeGraph`,
`Heatmap analysis: frequency-based file importance scoring.\n\nMost-accessed files get priority in context filling.\n\n${U}\n\n#AI #DataDriven`,
`Deduplication engine: cross-file shared content detection.\n\nImport blocks, boilerplate, config — deduplicated.\n\n${U}\n\n#AI #Compression`,
`Fill algorithm: knapsack-style context packing.\n\nMaximize information value within a token budget.\n\n${U}\n\n#AI #Algorithms`,
`Structured output: JSON, markdown, compressed formats.\n\nEvery tool returns machine-parseable output.\n\n${U}\n\n#AI #MCP`,
`150+ integration tests. CI on every commit.\n\nQuality is not optional.\n\n${G}\n\n#Rust #Testing #CI`,
],

// ── SLOT 8 (18:00 UTC) — COMMUNITY / SOCIAL PROOF ───────────────
community: [
`{{STARS}} stars on GitHub. Growing every day.\n\nJoin the lean-ctx community.\n\n${G}\n\n#OpenSource #AI #DevTools`,
`{{DISCORD}} developers on Discord.\n\nGet help, share tips, request features.\n\n${U}\n\n#Discord #AI #Community`,
`{{FORKS}} forks — developers are extending lean-ctx.\n\nWhat will you build?\n\n${G}\n\n#OpenSource #AI`,
`{{ISSUES}} issues resolved on GitHub.\n\nWe ship fixes fast. Usually same day.\n\n${G}\n\n#OpenSource #AI`,
`6 contributors building the future of AI context.\n\nPRs welcome. Every contribution matters.\n\n${G}\n\n#OpenSource #AI`,
`lean-ctx is fully open source. Apache 2.0.\n\nRead every line of code. Build trust through transparency.\n\n${G}\n\n#OpenSource #Transparency`,
`{{NPM_DL}} npm downloads this month.\n\nThe JavaScript community has spoken.\n\n${U}\n\n#npm #AI #DevTools`,
`From 0 to {{STARS}} stars since launch.\n\nOrganic growth. No paid promotion. Just value.\n\n${U}\n\n#OpenSource #AI`,
`{{RELEASES}} releases shipped. More coming.\n\nWe listen to community feedback and iterate fast.\n\n${U}\n\n#AI #DevTools`,
`Join the Discord: real developers, real conversations.\n\nNo bots. No spam. Just context engineering.\n\n${U}\n\n#Discord #AI`,
`lean-ctx started as a side project.\n\nNow it's used by developers in 140+ countries.\n\n${U}\n\n#OpenSource #AI`,
`Star us on GitHub. Every star helps more developers discover lean-ctx.\n\n${G}\n\n#OpenSource #AI #DevTools`,
`We value every bug report.\n\n{{ISSUES}} issues fixed. Zero ignored.\n\n${G}\n\n#OpenSource #Community`,
`Community-requested features ship within days.\n\nThat's the open-source advantage.\n\n${U}\n\n#OpenSource #AI`,
`lean-ctx is built in public. Every commit is visible.\n\nTransparency is our policy.\n\n${G}\n\n#OpenSource #BuildInPublic`,
`Developers are switching from raw AI to lean-ctx enhanced AI.\n\n88% fewer tokens. Same results.\n\n${U}\n\n#AI #DevTools`,
`The lean-ctx Discord is the best place for context engineering tips.\n\n{{DISCORD}} members and growing.\n\n${U}\n\n#Discord #AI`,
`Contributors from around the world.\n\n6 developers. Multiple time zones. One mission.\n\n${G}\n\n#OpenSource #AI`,
`lean-ctx is the #1 context compression tool for AI coding.\n\n{{STARS}} stars. {{NPM_DL}} downloads/month. 88% savings.\n\n${U}\n\n#AI #DevTools`,
`New release? The Discord is the first to know.\n\nReal-time announcements. Live feedback.\n\n${U}\n\n#Discord #AI`,
`{{FORKS}} developers forked lean-ctx.\n\nCustom integrations. Private tools. Enterprise setups.\n\n${G}\n\n#OpenSource #Enterprise`,
`lean-ctx community: diverse editors, diverse languages, one goal.\n\nReduce AI token waste everywhere.\n\n${U}\n\n#AI #DevTools`,
`Every feature request gets a response.\n\nOpen issues. Transparent roadmap. Community-driven.\n\n${G}\n\n#OpenSource #AI`,
`lean-ctx has been featured in AI developer communities worldwide.\n\nJoin the movement.\n\n${U}\n\n#AI #DevTools`,
`{{CRATES_DL}} downloads on crates.io.\n\nRust developers know quality when they see it.\n\n${U}\n\n#Rust #AI`,
`We don't track you. We don't sell data.\n\nWe just compress context. That's it.\n\n${U}\n\n#Privacy #OpenSource`,
`Pull requests welcome.\n\nRust, TypeScript, documentation — every contribution helps.\n\n${G}\n\n#OpenSource #AI`,
`lean-ctx Discord FAQ covers everything.\n\nInstallation, troubleshooting, tips, integrations.\n\n${U}\n\n#Discord #AI`,
`The lean-ctx community helped close {{ISSUES}} issues.\n\nBug reports, reproductions, fixes — all community-driven.\n\n${G}\n\n#OpenSource`,
`{{STARS}} stars = {{STARS}} developers who believe in token-efficient AI.\n\nBe the next one.\n\n${G}\n\n#OpenSource #AI`,
],

// ── SLOT 9 (19:30 UTC) — QUICK INSTALL CTA ──────────────────────
installCTA: [
`Install lean-ctx in 60 seconds:\n\ncurl -fsSL ${U}/install.sh | sh\nlean-ctx setup\n\nDone.\n\n#AI #DevTools #OpenSource`,
`macOS users:\n\nbrew install lean-ctx\nlean-ctx setup\n\n88% token savings start now.\n\n${U}\n\n#macOS #AI #DevTools`,
`Rust developers:\n\ncargo install lean-ctx\nlean-ctx setup\n\nFrom one Rust project to another.\n\n${U}\n\n#Rust #AI #DevTools`,
`Node.js users:\n\nnpm i -g lean-ctx-bin\nlean-ctx setup\n\nWorks with every Node project.\n\n${U}\n\n#npm #AI #DevTools`,
`Arch Linux:\n\nyay -S lean-ctx\nlean-ctx setup\n\nAvailable on AUR.\n\n${U}\n\n#Linux #Arch #AI`,
`Try without installing:\n\nnpx lean-ctx-bin setup\n\nZero commitment. Full experience.\n\n${U}\n\n#AI #DevTools`,
`One command setup for {{EDITORS}} editors:\n\nlean-ctx setup\n\nCursor, Claude Code, Copilot — all configured automatically.\n\n${U}\n\n#AI #DevTools`,
`Already have lean-ctx? Update now:\n\nlean-ctx update\n\nLatest version: {{VERSION}}\n\n${U}\n\n#AI #DevTools`,
`Windows, macOS, or Linux — lean-ctx runs everywhere.\n\ncurl -fsSL ${U}/install.sh | sh\n\n#AI #DevTools #CrossPlatform`,
`5MB download. 60 seconds to install. 88% token savings.\n\nWhat are you waiting for?\n\n${U}\n\n#AI #DevTools`,
`Install via Homebrew and get automatic updates:\n\nbrew install lean-ctx\n\nAlways on the latest version.\n\n${U}\n\n#macOS #DevTools`,
`From zero to saving tokens in under a minute:\n\ncurl -fsSL ${U}/install.sh | sh && lean-ctx setup\n\n#AI #DevTools #OpenSource`,
`Cargo install for the Rust ecosystem:\n\ncargo install lean-ctx\n\n{{LOC}} lines of Rust. Built with care.\n\n${U}\n\n#Rust #AI`,
`Installing lean-ctx is the easiest way to cut AI costs:\n\n1. Install\n2. Run lean-ctx setup\n3. Save 88% on tokens\n\n${U}\n\n#AI #DevTools`,
`New to lean-ctx? Start here:\n\n${U}\n\nInstall, setup, save. It's that simple.\n\n#AI #DevTools #GetStarted`,
`lean-ctx works with your existing workflow.\n\nNo changes needed. Just install and forget.\n\n${U}\n\n#AI #DevTools`,
`Upgrade from any previous version:\n\nlean-ctx update\n\nSmooth upgrades. No breaking changes.\n\n${U}\n\n#AI #DevTools`,
`Install lean-ctx today. Save tokens tomorrow.\n\ncurl -fsSL ${U}/install.sh | sh\n\n#AI #DevTools #OpenSource`,
`Three ways to install:\n\ncurl | sh (universal)\nbrew (macOS)\ncargo (Rust)\n\nPick yours.\n\n${U}\n\n#AI #DevTools`,
`lean-ctx setup auto-detects your AI editors.\n\nNo manual config files. No JSON editing.\n\n${U}\n\n#AI #DevTools`,
`Global npm install:\n\nnpm i -g lean-ctx-bin\n\n{{NPM_DL}} downloads this month. Join them.\n\n${U}\n\n#npm #AI`,
`Every second you wait, your AI wastes tokens.\n\nInstall lean-ctx:\ncurl -fsSL ${U}/install.sh | sh\n\n#AI #DevTools`,
`Docker users can run lean-ctx too.\n\nSingle binary, copies into any container.\n\n${U}\n\n#Docker #AI #DevTools`,
`lean-ctx update checks for new versions automatically.\n\nOne command. Always current.\n\n${U}\n\n#AI #DevTools`,
`Install in CI/CD pipelines:\n\ncurl -fsSL ${U}/install.sh | sh\n\nCompress context in automated workflows too.\n\n#AI #CI #DevTools`,
`pi-lean-ctx on npm for platform-specific installs.\n\nOptimized for your architecture.\n\n${U}\n\n#npm #AI #DevTools`,
`First install? Run the benchmark after:\n\nlean-ctx benchmark\n\nSee your exact token savings.\n\n${U}\n\n#AI #DevTools`,
`lean-ctx is a single binary. No PATH conflicts. No dependency hell.\n\nJust works.\n\n${U}\n\n#AI #DevTools`,
`Uninstall cleanly if you don't like it:\n\nlean-ctx uninstall\n\nBut you won't want to. 88% savings.\n\n${U}\n\n#AI #DevTools`,
`Start saving tokens in 3 steps:\n\n1. curl -fsSL ${U}/install.sh | sh\n2. lean-ctx setup\n3. Code with AI\n\n#AI #DevTools`,
],

// ── SLOT 10 (21:00 UTC) — VISION / ROADMAP ──────────────────────
vision: [
`The future of AI coding is context-aware.\n\nlean-ctx is building that future today.\n\n${U}\n\n#AI #DevTools #Future`,
`Every AI model will get better. But context costs will always matter.\n\nlean-ctx optimizes the constant: context.\n\n${U}\n\n#AI #DevTools`,
`We're building the standard for AI context compression.\n\n{{TOOLS}} tools. 2 protocols. Growing ecosystem.\n\n${U}\n\n#AI #MCP #Standard`,
`lean-ctx today: 88% savings.\n\nGoal: make context a solved problem for every developer.\n\n${U}\n\n#AI #DevTools`,
`Agentic coding is the future. Context engineering makes it affordable.\n\nlean-ctx bridges the gap.\n\n${U}\n\n#AI #AgenticCoding`,
`From {{STARS}} stars to industry standard.\n\nThat's the lean-ctx roadmap.\n\n${U}\n\n#AI #DevTools #OpenSource`,
`Every AI editor will need context compression.\n\nlean-ctx is ready for all of them. Today.\n\n${U}\n\n#AI #DevTools`,
`Multi-agent workflows are coming.\n\nlean-ctx already supports agent coordination, handoff, and shared context.\n\n${U}\n\n#AI #MultiAgent`,
`The token economy is real. Compression is infrastructure.\n\nlean-ctx is the infrastructure layer.\n\n${U}\n\n#AI #Infrastructure`,
`AI models get smarter. Context windows get larger.\n\nBut efficient context always wins.\n\n${U}\n\n#AI #DevTools`,
`We believe context engineering is a new discipline.\n\nlean-ctx is the first tool built specifically for it.\n\n${U}\n\n#ContextEngineering #AI`,
`Open protocols: CCP for memory, CEP for efficiency.\n\nStandards that any tool can implement.\n\n${U}/protocols/\n\n#AI #OpenStandards`,
`Today: {{EDITORS}} editors. Tomorrow: every AI tool.\n\nlean-ctx is editor-agnostic by design.\n\n${U}\n\n#AI #DevTools`,
`The best code is the code your AI doesn't need to read.\n\nlean-ctx decides what matters.\n\n${U}\n\n#AI #DevTools`,
`We're making AI coding accessible to every developer.\n\n88% less cost means more developers can afford AI tools.\n\n${U}\n\n#AI #Accessibility`,
`lean-ctx v4 is coming.\n\nMore languages. Better compression. Faster caching.\n\n${U}\n\n#AI #DevTools #Roadmap`,
`Context compression is to AI what minification is to JavaScript.\n\nEssential infrastructure.\n\n${U}\n\n#AI #DevTools`,
`Our vision: zero-waste AI coding sessions.\n\nEvery token carries maximum information.\n\n${U}\n\n#AI #ZeroWaste`,
`lean-ctx is built for the long term.\n\nRust for safety. Apache 2.0 for freedom. Community for growth.\n\n${U}\n\n#OpenSource #AI`,
`In 2 years, you'll wonder how you coded with AI without context compression.\n\nlean-ctx is here now.\n\n${U}\n\n#AI #DevTools`,
`AI coding is a token economy. lean-ctx is the central bank.\n\nControlling inflation, one read at a time.\n\n${U}\n\n#AI #DevTools`,
`The overhead of AI is context. The solution is compression.\n\nlean-ctx proves it with 88% savings.\n\n${U}\n\n#AI #DevTools`,
`We're not just saving tokens. We're making AI coding sustainable.\n\n${U}\n\n#AI #Sustainability #DevTools`,
`Enterprise teams waste millions on AI tokens.\n\nlean-ctx at scale: 88% reduction across every developer.\n\n${U}\n\n#Enterprise #AI`,
`Tree-sitter for {{LANGS}} languages today.\n\nEvery major language tomorrow.\n\n${U}\n\n#AI #TreeSitter #Roadmap`,
`Context engineering will be as important as prompt engineering.\n\nlean-ctx leads both.\n\n${U}\n\n#AI #PromptEngineering`,
`The lean-ctx protocol suite: CCP + CEP.\n\nOpen standards for the agentic coding era.\n\n${U}/protocols/\n\n#AI #Protocols`,
`Building for a world where AI agents work 24/7.\n\nContext efficiency isn't optional — it's critical.\n\n${U}\n\n#AI #AgenticCoding`,
`Our promise: lean-ctx will always be open source.\n\nNo freemium. No lock-in. No telemetry.\n\n${G}\n\n#OpenSource #AI`,
`The context compression revolution starts here.\n\n{{STARS}} developers agree.\n\n${U}\n\n#AI #DevTools #OpenSource`,
],

// ── SLOT 11 (22:30 UTC) — ENGAGEMENT / QUESTION ─────────────────
engagement: [
`What's the most tokens your AI agent has burned in a single session?\n\nlean-ctx users report 88% savings.\n\n${U}\n\n#AI #DevTools`,
`How much do you spend on AI coding per month?\n\nlean-ctx could cut that by 88%.\n\n${U}\n\n#AI #SaveMoney`,
`Which AI editor do you use for coding?\n\nlean-ctx supports {{EDITORS}} of them.\n\n${U}\n\n#AI #DevTools`,
`Do you track your AI token usage?\n\nRun "lean-ctx gain" to see real savings data.\n\n${U}\n\n#AI #DevTools`,
`What's your biggest AI coding pain point?\n\nFor many devs it's token costs. lean-ctx fixes that.\n\n${U}\n\n#AI #DevTools`,
`Have you tried context compression for AI coding?\n\n88% savings. Real benchmark data.\n\n${U}/benchmark\n\n#AI #DevTools`,
`Cursor, Copilot, or Claude Code — which one do you prefer?\n\nlean-ctx works with all three.\n\n${U}\n\n#AI #DevTools`,
`How many times does your AI re-read the same file per session?\n\nWith lean-ctx, each re-read costs ~13 tokens.\n\n${U}\n\n#AI`,
`What programming language is your main focus?\n\nlean-ctx has tree-sitter support for {{LANGS}} languages.\n\n${U}\n\n#AI #DevTools`,
`Is your AI coding workflow efficient?\n\nMost developers waste 80%+ of their tokens on redundant reads.\n\n${U}\n\n#AI`,
`What would you do with $80/month saved on AI tokens?\n\nlean-ctx makes it possible.\n\n${U}\n\n#AI #SaveMoney`,
`Do you use MCP tools with your AI editor?\n\nlean-ctx provides {{TOOLS}} MCP tools for context optimization.\n\n${U}\n\n#MCP #AI`,
`How long does your AI take to understand a new codebase?\n\nctx_architecture: instant overview.\n\n${U}\n\n#AI #DevTools`,
`Rust, Go, Python, or TypeScript — what are you building?\n\nlean-ctx optimizes context for all of them.\n\n${U}\n\n#AI #DevTools`,
`Have you hit AI token limits during a coding session?\n\nlean-ctx keeps you under budget. Always.\n\n${U}\n\n#AI #DevTools`,
`What's your ideal AI coding setup?\n\nTell us and we'll show how lean-ctx fits in.\n\n${U}\n\n#AI #DevTools`,
`Do you compress your AI context? You should.\n\n88% savings with zero workflow changes.\n\n${U}\n\n#AI #DevTools`,
`How many editors have you tried for AI coding?\n\nlean-ctx supports {{EDITORS}} and counting.\n\n${U}\n\n#AI #DevTools`,
`Multi-agent or single agent workflows?\n\nlean-ctx supports both with ctx_agent coordination.\n\n${U}\n\n#AI #MultiAgent`,
`Are you ready for the agentic coding revolution?\n\nlean-ctx is the context layer that makes it affordable.\n\n${U}\n\n#AI #AgenticCoding`,
`What feature would you add to lean-ctx?\n\nOpen an issue — we build what the community needs.\n\n${G}\n\n#OpenSource #AI`,
`How important is privacy in your AI workflow?\n\nlean-ctx: zero telemetry, 100% local.\n\n${U}\n\n#Privacy #AI`,
`React, Vue, or Svelte — which framework?\n\nlean-ctx optimizes context for all frontend frameworks.\n\n${U}\n\n#AI #Frontend`,
`Terminal or GUI for AI coding?\n\nlean-ctx works everywhere. CLI, IDE, both.\n\n${U}\n\n#AI #DevTools`,
`Drop a star on GitHub if lean-ctx saved you tokens today.\n\n${G}\n\n#OpenSource #AI #DevTools`,
`Best AI coding moment this week?\n\nShare yours. We'll share how lean-ctx makes more of them possible.\n\n${U}\n\n#AI`,
`Do you review your AI's token consumption?\n\nlean-ctx gain shows session-by-session savings.\n\n${U}\n\n#AI #DevTools`,
`What's stopping you from trying lean-ctx?\n\n60-second install. Zero risk. 88% savings.\n\n${U}\n\n#AI #DevTools`,
`Biggest codebase you've used AI on?\n\nlean-ctx handles monorepos, microservices, and everything in between.\n\n${U}\n\n#AI`,
`What did your AI waste tokens on today?\n\nlean-ctx prevents exactly that.\n\n${U}\n\n#AI #DevTools`,
],

};

// ═══════════════════════════════════════════════════════════════════
// VALIDATION
// ═══════════════════════════════════════════════════════════════════

let errors = 0;
const categories = Object.entries(TWEETS);
let totalTweets = 0;

for (const [cat, pool] of categories) {
  if (pool.length !== 30) {
    console.error(`ERROR: ${cat} has ${pool.length} tweets (expected 30)`);
    errors++;
  }
  for (let i = 0; i < pool.length; i++) {
    totalTweets++;
    const maxLen = pool[i].replace(/\{\{[A-Z_]+\}\}/g, '999,999+').length;
    if (maxLen > 280) {
      console.error(`ERROR: ${cat}[${i}] could be ${maxLen} chars (max 280): "${pool[i].substring(0, 60)}..."`);
      errors++;
    }
    if (!pool[i].includes('leanctx.com') && !pool[i].includes('github.com/yvgude/lean-ctx')) {
      console.error(`ERROR: ${cat}[${i}] missing URL: "${pool[i].substring(0, 60)}..."`);
      errors++;
    }
  }
}

console.log(`Total tweets: ${totalTweets}`);
console.log(`Categories: ${categories.length}`);

if (errors > 0) {
  console.error(`\n${errors} ERRORS found. Fix before deploying.`);
  process.exit(1);
}

console.log('All tweets validated successfully.\n');

// ═══════════════════════════════════════════════════════════════════
// BUILD N8N WORKFLOW JSON
// ═══════════════════════════════════════════════════════════════════

// Node 1: Fetch live stats from APIs
const fetchStatsCode = `const https = require('https');

function fetchJSON(url) {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { 'User-Agent': 'lean-ctx-bot/1.0' } }, (res) => {
      let data = '';
      res.on('data', c => data += c);
      res.on('end', () => {
        try { resolve(JSON.parse(data)); }
        catch(e) { reject(new Error('Parse error: ' + data.substring(0, 100))); }
      });
    }).on('error', reject);
  });
}

const formatK = (n) => n >= 10000 ? Math.floor(n / 1000) + 'K+' : n >= 1000 ? (n / 1000).toFixed(1).replace('.0', '') + 'K+' : n + '';
const roundDown = (n, step) => Math.floor(n / step) * step;

try {
  const [gh, npmMain, npmPi, crates, discord] = await Promise.allSettled([
    fetchJSON('https://api.github.com/repos/yvgude/lean-ctx'),
    fetchJSON('https://api.npmjs.org/downloads/point/last-month/lean-ctx-bin'),
    fetchJSON('https://api.npmjs.org/downloads/point/last-month/pi-lean-ctx'),
    fetchJSON('https://api.crates.io/v1/crates/lean-ctx'),
    fetchJSON('https://discord.com/api/v9/invites/pTHkG9Hew9?with_counts=true'),
  ]);

  const ghData = gh.status === 'fulfilled' ? gh.value : {};
  const npmData = npmMain.status === 'fulfilled' ? npmMain.value : {};
  const npmPiData = npmPi.status === 'fulfilled' ? npmPi.value : {};
  const cratesData = crates.status === 'fulfilled' ? (crates.value.crate || {}) : {};
  const discordData = discord.status === 'fulfilled' ? discord.value : {};

  const stars = ghData.stargazers_count || 750;
  const forks = ghData.forks_count || 88;
  const npmDl = (npmData.downloads || 14000) + (npmPiData.downloads || 2700);
  const cratesDl = cratesData.downloads || 2800;
  const version = cratesData.newest_version || '3.3.2';
  const discordMembers = discordData.approximate_member_count || 167;

  return [{ json: {
    STARS: roundDown(stars, 50) + '+',
    FORKS: forks + '+',
    NPM_DL: formatK(npmDl),
    CRATES_DL: formatK(cratesDl),
    VERSION: version,
    DISCORD: discordMembers + '+',
    RELEASES: '130+',
    ISSUES: '100+',
    EDITORS: '21',
    TOOLS: '46',
    LANGS: '18',
    LOC: '97K+',
  }}];
} catch(e) {
  return [{ json: {
    STARS: '750+', FORKS: '88+', NPM_DL: '17K+', CRATES_DL: '2.8K+',
    VERSION: '3.3.2', DISCORD: '167+', RELEASES: '130+', ISSUES: '100+',
    EDITORS: '21', TOOLS: '46', LANGS: '18', LOC: '97K+',
  }}];
}`;

// Node 2: Pick tweet and fill templates
function buildPickerCode(tweets) {
  let code = '';
  code += `const stats = $input.first().json;\n`;
  code += `const hour = new Date().getUTCHours();\n`;
  code += `const dayOfYear = Math.floor((Date.now() - new Date(new Date().getFullYear(), 0, 0)) / 86400000);\n`;
  code += `const slotMap = {6:0,7:1,9:2,10:3,12:4,13:5,15:6,16:7,18:8,19:9,21:10,22:11};\n`;
  code += `const slot = slotMap[hour] !== undefined ? slotMap[hour] : 0;\n\n`;

  code += `const pools = [\n`;
  const cats = Object.values(tweets);
  for (let s = 0; s < cats.length; s++) {
    code += `  [\n`;
    for (const tweet of cats[s]) {
      const escaped = tweet.replace(/\\/g, '\\\\').replace(/'/g, "\\'").replace(/\n/g, '\\n');
      code += `    '${escaped}',\n`;
    }
    code += `  ],\n`;
  }
  code += `];\n\n`;

  code += `const pool = pools[slot];\n`;
  code += `const idx = dayOfYear % pool.length;\n`;
  code += `let tweet = pool[idx];\n\n`;
  code += `// Replace dynamic placeholders with live API data\n`;
  code += `Object.entries(stats).forEach(([key, val]) => {\n`;
  code += `  tweet = tweet.replace(new RegExp('\\\\{\\\\{' + key + '\\\\}\\\\}', 'g'), val);\n`;
  code += `});\n\n`;
  code += `if (tweet.length > 280) tweet = tweet.substring(0, 277) + '...';\n`;
  code += `return [{ json: { tweet, slot, dayOfYear, idx, charCount: tweet.length, stats } }];\n`;

  return code;
}

// Node 3: Post to Twitter (unchanged)
const twitterPostCode = `const crypto = require('crypto');
const https = require('https');

const CK = 'wZN4Nch9rHmtHC6Y33lrhe2b5';
const CS = 'm7UuVQAnZdd7H3a5Meoj3pdBNPdduRWnyWnwkHAcgkML8bq6DR';
const AT = '1999852637383217152-kwKk9mneIi0Cb48mQDkV58FRfmQujD';
const AS = '8RHZbwerv7BmQNFOTY8xNkwEpSlbwN0hjTHohgSp7puHN';

const tweet = $input.first().json.tweet;
const url = 'https://api.twitter.com/2/tweets';

function pctEnc(s) {
  return encodeURIComponent(s).replace(/[!'()*]/g, c => '%' + c.charCodeAt(0).toString(16).toUpperCase());
}

const ts = Math.floor(Date.now() / 1000).toString();
const nonce = crypto.randomBytes(16).toString('hex');

const op = {
  oauth_consumer_key: CK, oauth_nonce: nonce,
  oauth_signature_method: 'HMAC-SHA1', oauth_timestamp: ts,
  oauth_token: AT, oauth_version: '1.0'
};

const sp = Object.keys(op).sort().map(k => pctEnc(k) + '=' + pctEnc(op[k])).join('&');
const bs = 'POST&' + pctEnc(url) + '&' + pctEnc(sp);
const sk = pctEnc(CS) + '&' + pctEnc(AS);
const sig = crypto.createHmac('sha1', sk).update(bs).digest('base64');

const ah = 'OAuth ' + Object.entries({...op, oauth_signature: sig}).map(([k,v]) => k + '="' + pctEnc(v) + '"').join(', ');

const body = JSON.stringify({ text: tweet });

const result = await new Promise((resolve) => {
  const req = https.request(url, {
    method: 'POST',
    headers: {
      'Authorization': ah,
      'Content-Type': 'application/json',
      'Content-Length': Buffer.byteLength(body)
    }
  }, (res) => {
    let data = '';
    res.on('data', chunk => data += chunk);
    res.on('end', () => {
      try { resolve({ status: res.statusCode, body: JSON.parse(data) }); }
      catch(e) { resolve({ status: res.statusCode, body: data }); }
    });
  });
  req.on('error', err => resolve({ status: 0, error: err.message }));
  req.write(body);
  req.end();
});

return [{ json: { twitter: result, tweet, slot: $input.first().json.slot, idx: $input.first().json.idx } }];`;

const workflow = {
  name: "LeanCTX Daily Tips \u2192 Twitter",
  nodes: [
    {
      parameters: {
        rule: {
          interval: [
            { triggerAtHour: 6, triggerAtMinute: 0 },
            { triggerAtHour: 7, triggerAtMinute: 30 },
            { triggerAtHour: 9, triggerAtMinute: 0 },
            { triggerAtHour: 10, triggerAtMinute: 30 },
            { triggerAtHour: 12, triggerAtMinute: 0 },
            { triggerAtHour: 13, triggerAtMinute: 30 },
            { triggerAtHour: 15, triggerAtMinute: 0 },
            { triggerAtHour: 16, triggerAtMinute: 30 },
            { triggerAtHour: 18, triggerAtMinute: 0 },
            { triggerAtHour: 19, triggerAtMinute: 30 },
            { triggerAtHour: 21, triggerAtMinute: 0 },
            { triggerAtHour: 22, triggerAtMinute: 30 },
          ]
        }
      },
      id: "daily-trigger-001",
      name: "Schedule (12x Daily)",
      type: "n8n-nodes-base.scheduleTrigger",
      typeVersion: 1.2,
      position: [250, 300]
    },
    {
      parameters: {
        jsCode: fetchStatsCode
      },
      id: "fetch-stats-002",
      name: "Fetch Live Stats",
      type: "n8n-nodes-base.code",
      typeVersion: 2,
      position: [500, 300],
      onError: "continueRegularOutput"
    },
    {
      parameters: {
        jsCode: buildPickerCode(TWEETS)
      },
      id: "daily-pick-003",
      name: "Pick Viral Tweet",
      type: "n8n-nodes-base.code",
      typeVersion: 2,
      position: [750, 300]
    },
    {
      parameters: {
        jsCode: twitterPostCode
      },
      id: "daily-tweet-004",
      name: "Post to Twitter",
      type: "n8n-nodes-base.code",
      typeVersion: 2,
      position: [1000, 300],
      onError: "continueRegularOutput"
    }
  ],
  connections: {
    "Schedule (12x Daily)": {
      main: [[{ node: "Fetch Live Stats", type: "main", index: 0 }]]
    },
    "Fetch Live Stats": {
      main: [[{ node: "Pick Viral Tweet", type: "main", index: 0 }]]
    },
    "Pick Viral Tweet": {
      main: [[{ node: "Post to Twitter", type: "main", index: 0 }]]
    }
  },
  settings: {
    executionOrder: "v1"
  }
};

const outPath = path.join(__dirname, 'daily-tweets.json');
fs.writeFileSync(outPath, JSON.stringify(workflow, null, 2));
console.log(`Written to ${outPath}`);
console.log(`Fetch stats code: ${fetchStatsCode.length} chars`);
console.log(`Picker code: ${workflow.nodes[2].parameters.jsCode.length} chars`);
console.log(`Twitter code: ${twitterPostCode.length} chars`);
