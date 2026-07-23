import {
  Stack,
  Row,
  Grid,
  H1,
  H2,
  H3,
  Text,
  Card,
  CardHeader,
  CardBody,
  Pill,
  Stat,
  Callout,
  Divider,
  Code,
  Table,
  BarChart,
  CollapsibleSection,
  useHostTheme,
  useCanvasState,
} from "cursor/canvas";

type View = "overview" | "traits" | "chiptuner" | "progress" | "oss" | "layers" | "migration";

type PhaseStatus = "completed" | "active" | "pending" | "deferred";

const PHASES: { id: string; name: string; track: string; status: PhaseStatus; summary: string }[] = [
  { id: "P0", name: "IST-Hygiene", track: "A", status: "completed", summary: "BanditStore Keys, Gotcha-Lifecycle, Double-Pull Fix" },
  { id: "P1", name: "OCLA Contract", track: "A", status: "completed", summary: "14 Traits + Canonical Envelope + Errors + Discovery (R1-R4)" },
  { id: "P2", name: "OclaBus", track: "B", status: "completed", summary: "Globaler Bus mit bounded/no-op-Modus (R1-R4)" },
  { id: "P3", name: "Built-ins", track: "B", status: "completed", summary: "14 Traits, 15 Builtins, Registry, fail-closed Gates (R1-R4)" },
  { id: "P4", name: "Trait-Adoption", track: "A", status: "completed", summary: "14/14 Traits produktiv verdrahtet (R5)" },
  { id: "P5", name: "Unified Ledger", track: "B", status: "completed", summary: "Dual-Write, Budget Cascade, Binary Separation (R5-R10)" },
  { id: "P6", name: "Binary-Sep", track: "A", status: "deferred", summary: "In P0-P5 absorbiert" },
  { id: "P7", name: "Wire + SDK", track: "A", status: "completed", summary: "REST/gRPC/OpenAPI + Python/TS/Go SDKs + Contract Suite (R5-R10)" },
  { id: "P8", name: "Model Router", track: "C", status: "completed", summary: "Intent-Routing, Quality Gate, A/B-Test-Framework (R5-R10)" },
  { id: "P9", name: "Response Opt.", track: "C", status: "completed", summary: "Similarity-Dedup, Cache Bridge, Model-Aware Invalidation (R5-R10)" },
  { id: "P10", name: "Value Gate", track: "C", status: "pending", summary: "Privates Repo lean-ctx-enterprise (nicht Teil OSS)" },
  { id: "P11", name: "Deployment + A2A", track: "C", status: "completed", summary: "AgentGateway, CoW Capsules, Health, DLQ, Tracing (R7-R10)" },
];

const TRAITS: { phase: string; name: string; dim: string; status: string; desc: string }[] = [
  { phase: "OBSERVE", name: "ObservationHook", dim: "1", status: "LIVE", desc: "Request Lifecycle (pre/post)" },
  { phase: "OBSERVE", name: "UsageSink", dim: "1", status: "LIVE", desc: "Token-Counts und Kosten" },
  { phase: "OBSERVE", name: "MetricsExporter", dim: "1", status: "LIVE", desc: "JSON / Prometheus / OTEL" },
  { phase: "OBSERVE", name: "SavingsLedger", dim: "1", status: "LIVE", desc: "Ed25519-signierte Savings" },
  { phase: "OBSERVE", name: "IntentClassifier", dim: "3", status: "LIVE", desc: "Persona/Intent-Tag pro Request" },
  { phase: "OBSERVE", name: "OutcomeTracker", dim: "—", status: "LIVE", desc: "Output gemergt/gesendet/verworfen?" },
  { phase: "CONTROL", name: "CompressionProvider", dim: "1", status: "LIVE", desc: "Input-Kompression · ETPAO-netto" },
  { phase: "CONTROL", name: "ResponseOptimizer", dim: "2", status: "LIVE", desc: "Verbosity Control, Output-Cache" },
  { phase: "CONTROL", name: "ModelRouter", dim: "3", status: "LIVE", desc: "Kostenoptimales Modell pro Request" },
  { phase: "LEARN", name: "EfficiencyAnalyzer", dim: "—", status: "LIVE", desc: "Wo ist Verschwendung? (read-only)" },
  { phase: "ACT", name: "ConfigTuner", dim: "—", status: "LIVE", desc: "Config aendern (mit Approval)" },
  { phase: "ACT", name: "ExperimentRunner", dim: "—", status: "LIVE", desc: "A/B Tests + Auto-Rollback" },
  { phase: "SHARED", name: "ConnectorScheduler", dim: "—", status: "LIVE", desc: "Externe Daten (GH/GL/Jira)" },
  { phase: "SHARED", name: "AgentGateway", dim: "4", status: "LIVE", desc: "Capsules · Ownership · Chain Budget" },
];

function Zone(p: {
  x: number; y: number; w: number; h: number;
  label: string; color: string; labelY?: number;
}) {
  const t = useHostTheme();
  return (
    <g>
      <rect x={p.x} y={p.y} width={p.w} height={p.h} rx={10}
        fill={p.color} stroke={p.color} strokeWidth={1} opacity={0.12} />
      <rect x={p.x} y={p.y} width={p.w} height={p.h} rx={10}
        fill="none" stroke={p.color} strokeWidth={1.5} opacity={0.4} />
      {p.label && (
        <text x={p.x + 12} y={p.labelY ?? p.y + 16} fill={t.text.secondary}
          fontSize={9} fontWeight={700} fontFamily="Inter, system-ui, sans-serif"
          letterSpacing={0.8}>{p.label}</text>
      )}
    </g>
  );
}

function Box(p: {
  x: number; y: number; w: number; h: number;
  label: string; sub?: string;
  fill: string; stroke: string; fg: string;
  radius?: number; dash?: boolean; fontSize?: number;
}) {
  const t = useHostTheme();
  return (
    <g>
      <rect x={p.x} y={p.y} width={p.w} height={p.h} rx={p.radius ?? 5}
        fill={p.fill} stroke={p.stroke} strokeWidth={1.5}
        strokeDasharray={p.dash ? "6 3" : undefined} />
      <text x={p.x + p.w / 2} y={p.sub ? p.y + p.h / 2 - 5 : p.y + p.h / 2 + 1}
        textAnchor="middle" dominantBaseline="middle"
        fill={p.fg} fontSize={p.fontSize ?? 10} fontWeight={600}
        fontFamily="Inter, system-ui, sans-serif">{p.label}</text>
      {p.sub && (
        <text x={p.x + p.w / 2} y={p.y + p.h / 2 + 8}
          textAnchor="middle" dominantBaseline="middle"
          fill={t.text.tertiary} fontSize={8}
          fontFamily="Inter, system-ui, sans-serif">{p.sub}</text>
      )}
    </g>
  );
}

function Arrow(p: {
  x1: number; y1: number; x2: number; y2: number;
  color: string; dash?: boolean; label?: string;
}) {
  const t = useHostTheme();
  const dx = p.x2 - p.x1, dy = p.y2 - p.y1;
  const len = Math.sqrt(dx * dx + dy * dy);
  if (len === 0) return null;
  const ux = dx / len, uy = dy / len;
  const tx = p.x2 - ux * 5, ty = p.y2 - uy * 5;
  return (
    <g>
      <line x1={p.x1} y1={p.y1} x2={tx} y2={ty}
        stroke={p.color} strokeWidth={1.5} opacity={0.5}
        strokeDasharray={p.dash ? "5 3" : undefined} />
      <polygon
        points={`${p.x2},${p.y2} ${tx - uy * 3},${ty + ux * 3} ${tx + uy * 3},${ty - ux * 3}`}
        fill={p.color} opacity={0.5} />
      {p.label && (
        <text x={(p.x1 + p.x2) / 2 + 6} y={(p.y1 + p.y2) / 2 - 5}
          textAnchor="middle" fill={t.text.tertiary}
          fontSize={7} fontFamily="Inter, system-ui, sans-serif">{p.label}</text>
      )}
    </g>
  );
}

function OverviewDiagram() {
  const t = useHostTheme();
  const gr = t.category.green;
  const bl = t.category.blue;
  const pu = t.category.purple;
  const or = t.category.orange;
  const pk = t.category.pink;
  const n = t.fill.secondary;
  const b = t.stroke.secondary;
  const f1 = t.text.primary;
  const f2 = t.text.secondary;
  const f3 = t.text.tertiary;
  const W = 940, H = 700;

  return (
    <svg width={W} height={H} viewBox={`0 0 ${W} ${H}`} style={{ display: "block" }}>
      {/* Agents */}
      <Zone x={200} y={8} w={540} h={40} label="AGENTS & APPS" color={b} />
      {["Cursor", "Claude Code", "Copilot", "Custom Agents", "Multi-Agent"].map((a, i) => (
        <g key={a}><Box x={215 + i * 104} y={18} w={92} h={22} label={a} fill={n} stroke={b} fg={f1} fontSize={9} /></g>
      ))}

      {/* Schicht 5: AI Value Gate */}
      <Zone x={600} y={60} w={310} h={130} label="SCHICHT 5: AI VALUE GATE / ENTERPRISE" color={pk} />
      <rect x={820} y={62} width={80} height={14} rx={7} fill={pk} fillOpacity={0.2} stroke={pk} strokeWidth={1} />
      <text x={860} y={72} textAnchor="middle" fill={pk} fontSize={7} fontWeight={700} fontFamily="Inter, system-ui, sans-serif">COMMERCIAL</text>
      <Box x={615} y={82} w={138} h={30} label="AI Value Multiple" sub="CEO/CFO: eine Zahl" fill={n} stroke={pk} fg={f1} />
      <Box x={760} y={82} w={138} h={30} label="LEARN + ACT" sub="Automation" fill={n} stroke={pk} fg={f1} />
      <Box x={615} y={120} w={138} h={30} label="Team Dashboard" sub="Aggregation + Insights" fill={n} stroke={pk} fg={f1} />
      <Box x={760} y={120} w={138} h={30} label="Value Share" sub="Quality + Approval + Cap" fill={n} stroke={pk} fg={f1} />
      <Box x={615} y={158} w={283} h={22} label="Setup + negotiated License + Support + optional Value Share" fill={n} stroke={pk} fg={f2} fontSize={8} />

      {/* Schicht 4: Interception Points */}
      <Zone x={30} y={60} w={555} h={90} label="SCHICHT 4: INTERCEPTION POINTS (alle OSS)" color={gr} />
      <Box x={45} y={82} w={170} h={56} label="LeanCTX" sub="Shell Hook + MCP Server" fill={n} stroke={gr} fg={f1} />
      <Box x={222} y={82} w={170} h={56} label="Lean Embed" sub="/v1 API + SDKs" fill={n} stroke={gr} fg={f1} />
      <Box x={400} y={82} w={170} h={56} label="Lean OS" sub="Enterprise Surface (OSS)" fill={n} stroke={gr} fg={f1} />

      {/* Schicht 3: Unified Ledger */}
      <Zone x={30} y={165} w={555} h={55} label="SCHICHT 3: UNIFIED LEDGER" color={or} />
      {["Ed25519 + SHA-256", "Intent + Outcome", "Model + Routing", "Agent Chain"].map((l, i) => (
        <g key={l}><Box x={45 + i * 135} y={183} w={128} h={28} label={l} fill={n} stroke={or} fg={f2} fontSize={9} /></g>
      ))}

      <Arrow x1={307} y1={142} x2={307} y2={165} color={or} label="schreibt in" />
      <Arrow x1={585} y1={180} x2={615} y2={140} color={pk} dash label="speist" />

      {/* Schicht 2: OCLA Contract */}
      <Zone x={30} y={235} w={880} h={175} label="SCHICHT 2: OCLA CONTRACT — 14 TRAITS, 4 DIMENSIONEN (OSS)" color={bl} />

      {/* Dimension labels */}
      <text x={45} y={262} fill={bl} fontSize={8} fontWeight={700} fontFamily="Inter, system-ui, sans-serif">OBSERVE (6)</text>
      {["ObservationHook", "UsageSink", "MetricsExporter", "SavingsLedger"].map((tr, i) => (
        <g key={tr}><Box x={45 + i * 130} y={268} w={124} h={20} label={tr} fill={n} stroke={bl} fg={f2} fontSize={8} /></g>
      ))}
      {[
        { l: "IntentClassifier", tag: "NEU" },
        { l: "OutcomeTracker", tag: "NEU" },
      ].map((item, i) => (
        <g key={item.l}><Box x={565 + i * 170} y={268} w={160} h={20} label={`${item.l} (${item.tag})`} fill={n} stroke={bl} fg={bl} fontSize={8} dash /></g>
      ))}

      <text x={45} y={302} fill={gr} fontSize={8} fontWeight={700} fontFamily="Inter, system-ui, sans-serif">CONTROL (3) — 4 Dimensionen</text>
      <Box x={45} y={308} w={170} h={24} label="CompressionProvider" sub="DIM 1: Input" fill={n} stroke={gr} fg={f1} fontSize={9} />
      <Box x={222} y={308} w={170} h={24} label="ResponseOptimizer" sub="DIM 2: Output" fill={n} stroke={gr} fg={f1} fontSize={9} />
      <Box x={400} y={308} w={170} h={24} label="ModelRouter" sub="DIM 3: Routing" fill={n} stroke={gr} fg={f1} fontSize={9} />

      <text x={45} y={348} fill={or} fontSize={8} fontWeight={700} fontFamily="Inter, system-ui, sans-serif">LEARN (1) + ACT (2)</text>
      <Box x={45} y={354} w={170} h={20} label="EfficiencyAnalyzer" fill={n} stroke={or} fg={f2} fontSize={8} />
      <Box x={222} y={354} w={170} h={20} label="ConfigTuner" fill={n} stroke={pu} fg={f2} fontSize={8} />
      <Box x={400} y={354} w={170} h={20} label="ExperimentRunner" fill={n} stroke={pu} fg={f2} fontSize={8} />

      <text x={45} y={390} fill={b} fontSize={8} fontWeight={700} fontFamily="Inter, system-ui, sans-serif">SHARED (2)</text>
      <Box x={45} y={396} w={170} h={20} label="ConnectorScheduler" fill={n} stroke={b} fg={f2} fontSize={8} />
      <Box x={222} y={396} w={265} h={20} label="AgentGateway — DIM 4: Agent-to-Agent" fill={n} stroke={b} fg={f1} fontSize={8} />

      {/* Schicht 1: Engine */}
      <Zone x={30} y={425} w={555} h={130} label="SCHICHT 1: THE ENGINE — lean-ctx-core (OSS)" color={gr} />
      <Box x={45} y={447} w={170} h={28} label="MCP Server" sub="81+ Tools, stdio" fill={n} stroke={gr} fg={f1} />
      <Box x={222} y={447} w={170} h={28} label="Proxy" sub="HTTP Forward + Compress" fill={n} stroke={gr} fg={f1} />
      <Box x={400} y={447} w={170} h={28} label="CLI" sub="70+ Commands" fill={n} stroke={b} fg={f1} />
      {[
        "Shell Compression (95+ Patterns)",
        "Session Cache · Memory · CCP",
        "BM25 + Graph Index (4 Layer)",
        "Hooks (20 Agents) · Daemon · IPC",
      ].map((m, i) => (
        <g key={m}><Box x={45} y={482 + i * 18} w={525} h={15} label={m} fill={n} stroke={b} fg={f3} fontSize={8} /></g>
      ))}

      {/* Arrows top to bottom */}
      {[0, 1, 2, 3, 4].map(i => (
        <g key={`ta${i}`}><Arrow x1={261 + i * 104} y1={40} x2={307} y2={60} color={b} /></g>
      ))}
      <Arrow x1={307} y1={150} x2={307} y2={165} color={or} />
      <Arrow x1={307} y1={220} x2={307} y2={235} color={bl} />
      <Arrow x1={307} y1={416} x2={307} y2={425} color={gr} />

      {/* Cloud Server */}
      <Zone x={600} y={210} w={310} h={50} label="CLOUD — api.leanctx.com" color={pk} />
      <text x={755} y={226} textAnchor="middle" fill={f3} fontSize={8} fontFamily="Inter, system-ui, sans-serif">(optional: Auth · Billing · Updates · Support)</text>
      <Box x={615} y={236} w={282} h={16} label="Keine Token-/Ledger-Daten-Abhaengigkeit" fill={n} stroke={pk} fg={f3} fontSize={8} />

      {/* LLM Providers */}
      <Zone x={150} y={575} w={640} h={45} label="" color={b} />
      {["Guenstig (Haiku)", "Standard (Sonnet)", "Premium (Opus)", "Local (Ollama)"].map((p, i) => (
        <g key={p}><Box x={165 + i * 155} y={583} w={140} h={28} label={p} fill={n} stroke={b} fg={f3} fontSize={9} /></g>
      ))}
      <Arrow x1={307} y1={555} x2={307} y2={575} color={b} dash label="DIM 3: richtiges Modell" />

      {/* Bottom label */}
      <text x={470} y={645} textAnchor="middle" fill={gr} fontSize={10} fontWeight={700}
        fontFamily="Inter, system-ui, sans-serif">
        OBSERVE → CONTROL → LEARN → ACT → OBSERVE (geschlossener Loop)
      </text>
      <text x={470} y={660} textAnchor="middle" fill={f3} fontSize={9}
        fontFamily="Inter, system-ui, sans-serif">
        4 Dimensionen: Input · Output · Routing · Agent-to-Agent | ETPAO + Quality Gate | Zero Telemetry
      </text>

      {/* Dimension badges */}
      {[
        { x: 600, y: 290, label: "DIM 1: Input", color: gr },
        { x: 600, y: 310, label: "DIM 2: Output", color: gr },
        { x: 600, y: 330, label: "DIM 3: Routing", color: gr },
        { x: 600, y: 350, label: "DIM 4: Agent", color: b },
      ].map(d => (
        <g key={d.label}>
          <rect x={d.x} y={d.y} width={85} height={15} rx={7} fill={d.color} fillOpacity={0.15} stroke={d.color} strokeWidth={1} />
          <text x={d.x + 42} y={d.y + 10} textAnchor="middle" fill={d.color} fontSize={7} fontWeight={600}
            fontFamily="Inter, system-ui, sans-serif">{d.label}</text>
        </g>
      ))}
      <text x={690} y={300} fill={f3} fontSize={7} fontFamily="Inter, system-ui, sans-serif">Gebaut</text>
      <text x={690} y={320} fill={f3} fontSize={7} fontFamily="Inter, system-ui, sans-serif">P9</text>
      <text x={690} y={340} fill={f3} fontSize={7} fontFamily="Inter, system-ui, sans-serif">P8</text>
      <text x={690} y={360} fill={f3} fontSize={7} fontFamily="Inter, system-ui, sans-serif">P11</text>
    </svg>
  );
}

function OverviewView() {
  return (
    <Stack gap={20}>
      <Callout tone="info" title="Customer-owned Token-Control-Platform">
        Token Data Plane · Token Control Plane · Token Value & Evidence Plane. Alle operativen Daten bleiben beim Kunden; Thinkery monetarisiert Integration, Enterprise Subscription, Support und verifizierte Netto-Savings.
      </Callout>
      <div style={{ overflowX: "auto" }}><OverviewDiagram /></div>
      <Grid columns={5} gap={12}>
        <Stat value="5" label="Schichten" />
        <Stat value="14" label="OCLA Traits" />
        <Stat value="4" label="Dimensionen" />
        <Stat value="3" label="Interception Points" />
        <Stat value="0/11" label="GA Gates belegt" />
      </Grid>
    </Stack>
  );
}

function TraitsView() {
  return (
    <Stack gap={20}>
      <H2>14 OCLA Traits — 4 Kontroll-Dimensionen</H2>
      <Text tone="secondary">Jeder Token der zwischen LLM und Agent fliesst wird durch diese Traits kontrolliert. 9 existieren, 5 sind neu.</Text>

      <Table
        headers={["OCLA-Phase", "Trait", "DIM", "Status", "Funktion"]}
        rows={TRAITS.map(tr => [
          tr.phase,
          tr.name,
          tr.dim,
          tr.status,
          tr.desc,
        ])}
        rowTone={TRAITS.map(tr => tr.status === "NEU" ? "info" : undefined)}
        striped
      />

      <H3>Die 4 Dimensionen — Chiptuner-Analogie</H3>
      <Grid columns={2} gap={16}>
        <Card>
          <CardHeader trailing={<Pill size="sm" tone="success">GEBAUT</Pill>}>DIM 1: Einspritzung (Input)</CardHeader>
          <CardBody>
            <Stack gap={4}>
              <Text size="small">Alles was zum Modell geht: Handles/Deltas, Datei-Reads, Shell-Output und Retrieval. Ziel: minimale ETPAO bei gleichem Quality Gate.</Text>
              <Text size="small" tone="secondary">Trait: <Code>CompressionProvider</Code></Text>
            </Stack>
          </CardBody>
        </Card>
        <Card>
          <CardHeader trailing={<Pill size="sm">Phase 9</Pill>}>DIM 2: Abgas (Output)</CardHeader>
          <CardBody>
            <Stack gap={4}>
              <Text size="small">Output-Tokens sind 3-5x teurer. Verbosity Control, Response-Cache, Dedup. Niemand sonst tut das.</Text>
              <Text size="small" tone="secondary">Trait: <Code>ResponseOptimizer</Code></Text>
            </Stack>
          </CardBody>
        </Card>
        <Card>
          <CardHeader trailing={<Pill size="sm">Phase 8</Pill>}>DIM 3: Leistungsstufe (Routing)</CardHeader>
          <CardBody>
            <Stack gap={4}>
              <Text size="small">IntentClassifier wählt die kleinste qualifizierte Modell-/Effort-Stufe; Escalation nur bei Unsicherheit oder fehlgeschlagenem Quality Gate.</Text>
              <Text size="small" tone="secondary">Traits: <Code>ModelRouter</Code> + <Code>IntentClassifier</Code></Text>
            </Stack>
          </CardBody>
        </Card>
        <Card>
          <CardHeader trailing={<Pill size="sm">Phase 11</Pill>}>DIM 4: Mehrzylinder (Agent-to-Agent)</CardHeader>
          <CardBody>
            <Stack gap={4}>
              <Text size="small">Multi-Agent: Context Capsules, Delta-Handoffs, Ownership Leases, bounded Work Graph und Chain Budgets.</Text>
              <Text size="small" tone="secondary">Trait: <Code>AgentGateway</Code></Text>
            </Stack>
          </CardBody>
        </Card>
      </Grid>
    </Stack>
  );
}

function ChiptunerView() {
  return (
    <Stack gap={20}>
      <H2>Enterprise Economics: Plattformgebuehr + verifizierter Wert</H2>

      <Callout tone="success" title="Das Prinzip">
        Value Share gilt nur auf vereinbarte Netto-Savings: Baseline, Messmethode, Quality Gate, exklusive Attribution, Customer Approval und Cap. Ed25519 beweist nur Integritaet und Herkunft.
      </Callout>

      <Text size="small" tone="secondary">Revenue: Setup & Integration + Enterprise Subscription + Support/LTS + optionaler gedeckelter Value Share. Nur das freigegebene Customer Schedule ist preislich verbindlich.</Text>

      <Grid columns={4} gap={12}>
        <Stat value="4" label="Revenue-Komponenten" />
        <Stat value="0" label="Core-Funktionen gegated" tone="success" />
        <Stat value="100%" label="Customer Data Ownership" />
        <Stat value="Optional" label="Value Share" tone="info" />
      </Grid>

      <Grid columns="1fr 1fr" gap={16}>
        <Card>
            <CardHeader>Value Evidence nach Dimension</CardHeader>
          <CardBody>
            <BarChart
              categories={["DIM 1: Input (70%)", "DIM 3: Routing (40%)", "DIM 2: Output (30%)"]}
              series={[{ name: "Illustrativer Anteil", data: [70, 20, 10], tone: "success" }]}
              horizontal
              height={180}
            />
            <Text size="small" tone="tertiary">Nur Szenario: reale Anteile benötigen Baseline, Quality Gate und exklusive Attribution.</Text>
          </CardBody>
        </Card>
        <Card>
            <CardHeader>Commercial Decision Chain</CardHeader>
          <CardBody>
            <BarChart
              categories={["Measured", "Quality-passed", "Approved", "Settled"]}
              series={[{ name: "Illustrative Evidence Funnel", data: [100, 85, 75, 70], tone: "info" }]}
              height={180}
            />
            <Text size="small" tone="tertiary">Nur approved und settled Net Savings sind abrechnungsrelevant.</Text>
          </CardBody>
        </Card>
      </Grid>

      <H3>Commercial Components</H3>
      <Table
        headers={["Komponente", "Pricing", "Was ist enthalten", "Target"]}
        rows={[
          ["Open Platform", "$0 / Apache-2.0", "Engine + OCLA + Data Plane + Ledger/Verifier", "Developer / Self-hoster"],
          ["AI Value Gate", "Negotiated", "Org Control, Value Intelligence, Assurance, Settlement", "Enterprise"],
          ["Services + Value Share", "Scope / Customer Schedule", "Setup, Integration, Support, optional capped Value Share", "Enterprise"],
        ]}
        rowTone={[undefined, "success", "info"]}
        striped
      />

      <H3>Kategorievergleich</H3>
      <Table
        headers={["Kategorie", "Customer-owned?", "Komprimiert?", "Kontrolliert?", "Evidence?", "Wertmodell?"]}
        rows={[
          ["Model Provider", "teilweise", "nein", "eigene Modelle", "Usage", "Seats / Usage"],
          ["Generic Gateway", "oft", "selten", "Routing / Policy", "Logs / Metrics", "License"],
          ["Observability", "variiert", "nein", "beobachtet", "Telemetry", "Usage"],
          ["LeanCTX Platform", "ja", "Input/Output/A2A", "4 Dimensionen", "Quality-bound Ledger", "License + optional Value"],
        ]}
        rowTone={[undefined, undefined, undefined, "success"]}
        striped
      />

      <Callout tone="info" title="Pricing Policy">
        <Text size="small">Enterprise ist negotiated. Prozent-, Seat- und Garantiebeispiele sind erst durch ein freigegebenes Customer Schedule verbindlich.</Text>
      </Callout>
    </Stack>
  );
}

function ProgressView() {
  const t = useHostTheme();
  return (
    <Stack gap={20}>
      <H2>Fortschritt: Premium Program + OCLA Work-Packages</H2>
      <Text tone="secondary">W0–W10 führen bis GA; P0–P11 liefern die technischen OCLA-Pakete.</Text>

      <Grid columns={4} gap={12}>
        <Stat value="W0-W10" label="ALL COMPLETE" tone="success" />
        <Stat value="P0-P11" label="10/11 DONE" tone="success" />
        <Stat value="14/14" label="Traits LIVE" tone="success" />
        <Stat value="419+" label="Kernel Tests" tone="success" />
      </Grid>

      <Table
        headers={["Phase", "Track", "Was", "Status"]}
        rows={PHASES.map(p => [p.id, `Track ${p.track}`, `${p.name}: ${p.summary}`, p.status])}
        rowTone={PHASES.map(p => p.status === "completed" ? "success" : p.status === "pending" ? "info" : p.status === "deferred" ? "neutral" : undefined)}
        striped
      />

      <H3>Kritische Pfade</H3>
      <Callout tone="success" title="OSS OCLA: P0-P9, P11 COMPLETE (R1-R30)">
        <Text size="small">Context Kernel LIVE in allen Hot-Paths. 419+ Tests, 0 Clippy Warnings. Waves W0-W10 abgeschlossen. Nur P10 (AI Value Gate) offen — privates Repo.</Text>
      </Callout>

      <H3>Track-Abhaengigkeiten</H3>
      <Table
        headers={["Phase", "Braucht", "Kann parallel zu"]}
        rows={[
          ["P0 IST-Hygiene", "—", "—"],
          ["P1 OCLA Contract (14 Traits)", "P0", "—"],
          ["P2 OclaBus + Emitters", "P1", "P4"],
          ["P3 Built-ins (inkl. Intent+Outcome)", "P2", "P4"],
          ["P4 Trait-Adoption", "P1", "P2, P3"],
          ["P5 Unified Ledger", "P3", "P4"],
          ["P6 Binary-Separation", "P4", "P5 (deferred)"],
          ["P7 Wire + SDKs", "P1; Pilot: P2/P5", "P3, P4"],
          ["P8 Model Router (DIM 3)", "P5", "P9"],
          ["P9 Response Optimizer (DIM 2)", "P5", "P8"],
          ["P10 AI Value Gate v0", "P8, P9", "P11"],
          ["P11 Deployment Surface + AgentGateway", "Adoption/Naming Gate", "W6–W8"],
        ]}
        rowTone={["success", "success", "success", "success", "success", "success", "neutral", "success", "success", "success", "info", "success"]}
        striped
      />
    </Stack>
  );
}

function OSSView() {
  return (
    <Stack gap={20}>
      <H2>OSS vs. Commercial — eine einzige Grenze</H2>

      <Callout tone="success" title="Strategisches Prinzip: Oeffne die Pipe, verkaufe Enterprise-Skalierung und Wert">
        Open Source schafft Distribution, Pruefbarkeit und Integrationsstandard. Kundendaten bleiben beim Kunden. Commercial sind Control Plane, Value Intelligence, Assurance, LTS, SLA, Support und verifizierter Value Share.
      </Callout>

      <Grid columns={2} gap={16}>
        <Card>
          <CardHeader trailing={<Pill size="sm" tone="success">Apache-2.0</Pill>}>ALLES Open Source</CardHeader>
          <CardBody>
            <Stack gap={6}>
              <Text size="small" weight="semibold">lean-ctx-core — Der Motor</Text>
              <Text size="small" tone="secondary">81+ MCP Tools, Proxy, CLI, Shell Engine, Caches, Indexes, Hooks, Dashboard</Text>
              <Divider />
              <Text size="small" weight="semibold">lean-ctx-ocla — 14 Traits</Text>
              <Text size="small" tone="secondary">Rust Traits + Wire Contract + Capability Discovery + Contract Suite.</Text>
              <Divider />
              <Text size="small" weight="semibold">Lean Embed — SDK / API</Text>
              <Text size="small" tone="secondary">Java, .NET, Python, TypeScript, Go und Rust SDKs. /v1 API.</Text>
              <Divider />
              <Text size="small" weight="semibold">Open Org/Team Gateway Run-Modes</Text>
              <Text size="small" tone="secondary">Customer-owned, Apache-2.0, local-free; kein separates Produkt oder Lizenz-Tier.</Text>
              <Divider />
              <Text size="small" weight="semibold">Unified Ledger</Text>
              <Text size="small" tone="secondary">Ed25519-signiert, hash-verkettet, lokal verifizierbar</Text>
            </Stack>
          </CardBody>
        </Card>
        <Card>
          <CardHeader trailing={<Pill size="sm">Commercial</Pill>}>Enterprise Subscription</CardHeader>
          <CardBody>
            <Stack gap={6}>
              <Text size="small" weight="semibold">AI Value Gate</Text>
              <Text size="small" tone="secondary">Customer-owned Value Intelligence, Governance, Fleet/Policy Management, Assurance und Executive Views.</Text>
              <Divider />
              <Text size="small" weight="semibold">Warum Kunden zahlen</Text>
              <Text size="small" tone="secondary">Fuer Enterprise-Skalierung, zertifizierten Betrieb, Support und nachweisbaren Wert — nicht fuer Datenzugriff oder lokale Kernfunktion.</Text>
              <Divider />
              <Text size="small" weight="semibold">Repo: root/lean-ctx-enterprise</Text>
              <Text size="small" tone="secondary">Proprietäre Enterprise-Surface; konsumiert ausschließlich versionierte öffentliche Contracts.</Text>
            </Stack>
          </CardBody>
        </Card>
      </Grid>

      <Table
        headers={["Repository / Artefakt", "Lizenz", "Inhalt", "Boundary"]}
        rows={[
          ["lean-ctx-core", "Apache-2.0", "Engine + MCP + Proxy + CLI", "Adoption, Vertrauen, Distribution"],
          ["lean-ctx-ocla", "Apache-2.0", "14 Traits + Wire Contract + Contract Suite", "Technologieuebergreifende Integration"],
          ["lean-ctx-embed", "Apache-2.0", "/v1 API + Java/.NET/Python/TS/Go/Rust", "Entwickler bauen Services damit"],
          ["Org/Team Gateway", "Apache-2.0", "Customer-owned Run-Modes im OSS-Repo", "Local-Free und unabhängig"],
          ["root/lean-ctx-enterprise", "Proprietary", "Enterprise Control Plane + AI Value Gate", "Wire/SDK Contract only"],
        ]}
        rowTone={[undefined, undefined, undefined, undefined, "info"]}
        striped
      />
    </Stack>
  );
}

function LayersView() {
  return (
    <Stack gap={20}>
      <H2>5 Schichten im Detail</H2>

      <CollapsibleSection title="Schicht 1: The Engine (OSS)" defaultOpen>
        <Stack gap={8}>
          <Text size="small">Was LeanCTX heute schon ist: lokale Context-, Compression-, Memory-, MCP-, Proxy- und Tool-Runtime.</Text>
          <Table
            headers={["Komponente", "Pfad", "Funktion"]}
            rows={[
              ["MCP Server", "server/", "81+ Tools fuer Cursor/Claude/VS Code"],
              ["Proxy", "proxy/", "HTTP Forward, Compression, Usage"],
              ["CLI", "cli/", "70+ Commands"],
              ["Shell Engine", "shell/", "95+ Kompressionsmuster"],
              ["Core Engine", "core/", "BM25, Graph, Session, Memory, Providers"],
              ["Hooks + Daemon", "hooks/, daemon.rs", "20 Agent Hooks, IPC, Autostart"],
            ]}
            striped
          />
        </Stack>
      </CollapsibleSection>

      <CollapsibleSection title="Schicht 2: OCLA Contract — 14 Traits (OSS)">
        <Text size="small">9 Proto-Logiken + 5 neue Capabilities. Object-safe Rust Traits plus versionierter Wire Contract, Capability Discovery und Contract Suite.</Text>
      </CollapsibleSection>

      <CollapsibleSection title="Schicht 3: Unified Ledger">
        <Stack gap={8}>
          <Text size="small">Jeder adressierbare Token-Fluss kann in EINEM lokalen Ledger attribuiert werden. Signaturen beweisen Integritaet/Herkunft; Messmethode, Qualitaet, Attribution und Approval bestimmen abrechenbaren Wert.</Text>
          <Table
            headers={["Feld", "Typ", "Quelle"]}
            rows={[
              ["baseline_tokens / actual_tokens", "u64", "CompressionProvider (existiert)"],
              ["net_saved_tokens", "u64", "Berechnet (existiert)"],
              ["intent_tag", "IntentTag enum", "IntentClassifier (NEU)"],
              ["outcome", "used|merged|sent|discarded", "OutcomeTracker (NEU)"],
              ["persona", "String", "IntentClassifier (NEU)"],
              ["model_original / model_routed", "String / Option", "ModelRouter (NEU)"],
              ["routing_savings", "u64", "ModelRouter (NEU)"],
              ["response_original / response_delivered", "u64", "ResponseOptimizer (NEU)"],
              ["agent_chain_id / chain_depth", "Option / u8", "AgentGateway (NEU)"],
              ["signature + hash_chain", "Ed25519 + SHA-256", "SavingsLedger (existiert)"],
              ["measurement + evidence", "direct|holdout|baseline|estimate|confirmed", "Messmethodik"],
              ["quality + attribution + approval", "typed", "Settlement"],
            ]}
            striped
          />
        </Stack>
      </CollapsibleSection>

      <CollapsibleSection title="Schicht 4: Interception Points (alle OSS)">
        <Table
          headers={["Point", "Was", "Target", "Lizenz"]}
          rows={[
            ["LeanCTX", "Shell Hook + MCP Server fuer Coding Agents", "Cursor, Claude Code, Copilot, etc.", "Apache-2.0"],
            ["Lean Embed", "/v1 API + Python/TS/Rust SDKs", "Custom Agents und Apps", "Apache-2.0"],
            ["Org Gateway Base", "Self-hosted, Auth, Postgres (ex KMU GW)", "Customer-managed Data Plane", "Apache-2.0"],
            ["Team Control Base", "RBAC-Basis, Aggregation, Connectors (ex Team Server)", "Self-hosted Teams", "Apache-2.0"],
            ["SDK / Sidecar / AgentGateway", "Wire Contract + Interception", "Custom Apps, Partner, A2A", "Apache-2.0"],
          ]}
          striped
        />
      </CollapsibleSection>

      <CollapsibleSection title="Schicht 5: AI Value Gate / Enterprise Subscription (Commercial)">
        <Stack gap={8}>
          <Text size="small">Customer-owned Control Plane, Value Intelligence, Governance, Assurance, zertifizierter Betrieb und Support. Monetarisierung: Setup + Subscription + Support + gedeckelter Verified Value Share.</Text>
          <Text size="small" weight="semibold">AI Value Multiple — 3 Baender:</Text>
          <Table
            headers={["Band", "Was", "Quelle"]}
            rows={[
              ["Approved (gruen)", "Quality-passed, reconciled Net Savings", "Customer-approved Evidence"],
              ["Estimated (gelb)", "Zeitersparnis, Akzeptanzrate", "Konfigurierbare Baseline + OutcomeTracker"],
              ["Not yet modeled (grau)", "Revenue-Impact, Qualitaet", "Zukunft — markiert, nicht reingeschummelt"],
            ]}
            striped
          />
        </Stack>
      </CollapsibleSection>
    </Stack>
  );
}

function MigrationView() {
  return (
    <Stack gap={20}>
      <H2>Migrationsplan: IST nach SOLL</H2>
      <Text tone="secondary">P0–P11 sind OCLA Work-Packages innerhalb des Premium-Programms W0–W10; sie sind nicht die vollständige GA-Planung.</Text>

      <Grid columns={3} gap={16}>
        <Card>
          <CardHeader>Track A — Infrastruktur</CardHeader>
          <CardBody>
            <Text size="small">P0 → P1 → P4 → P6 → P7</Text>
            <Text size="small" tone="secondary">Traits definieren, Module adaptieren, Binaries entkoppeln</Text>
          </CardBody>
        </Card>
        <Card>
          <CardHeader>Track B — OCLA-Kern</CardHeader>
          <CardBody>
            <Text size="small">P2 → P3 → P5</Text>
            <Text size="small" tone="secondary">Event-Bus, Built-in LEARN/ACT, Unified Ledger</Text>
          </CardBody>
        </Card>
        <Card>
          <CardHeader>Track C — Chiptuner</CardHeader>
          <CardBody>
            <Text size="small">P8 → P9 → P10 → P11</Text>
            <Text size="small" tone="secondary">Model Router, Response Opt., Value Gate, Deployment + AgentGateway</Text>
          </CardBody>
        </Card>
      </Grid>

      <H3>Repository- und Artefaktstruktur</H3>
      <Table
        headers={["Repository / Artefakt", "Lizenz", "Inhalt"]}
        rows={[
          ["lean-ctx-core", "Apache-2.0", "Engine: core/, server/, tools/, shell/, proxy/, cli/"],
          ["lean-ctx-ocla", "Apache-2.0", "14 Traits + Types + Events"],
          ["lean-ctx-embed", "Apache-2.0", "/v1 API + SDKs"],
          ["Org/Team Gateway", "Apache-2.0", "Offene Run-Modes in yvgude/lean-ctx"],
          ["root/lean-ctx-enterprise", "Proprietary", "AI Value Gate + Enterprise Control Plane"],
        ]}
        rowTone={[undefined, undefined, undefined, undefined, "info"]}
        striped
      />

      <Callout tone="info" title="Kernprinzip: Additive Layering">
        Kein bestehender Code wird entfernt oder umgeschrieben — nur gewrappt. Bestehende Tests bleiben unveraendert gruen. Alle 3 Tracks koennen teilweise parallel laufen.
      </Callout>
    </Stack>
  );
}

export default function OclaSollArchitektur() {
  const [view, setView] = useCanvasState<View>("view", "overview");
  return (
    <Stack gap={24} style={{ padding: 24, maxWidth: 1020 }}>
      <Stack gap={4}>
        <H1>Thinkery — Enterprise Token-Control-Platform</H1>
        <Text tone="secondary">
          3 Planes · 5 Schichten · 14 OCLA Capabilities · 4 Dimensionen · ETPAO · Customer-owned
        </Text>
      </Stack>
      <Row gap={8} wrap>
        <Pill active={view === "overview"} onClick={() => setView("overview")}>Gesamtbild</Pill>
        <Pill active={view === "traits"} onClick={() => setView("traits")}>14 Traits</Pill>
        <Pill active={view === "chiptuner"} onClick={() => setView("chiptuner")}>Chiptuner</Pill>
        <Pill active={view === "progress"} onClick={() => setView("progress")}>Fortschritt</Pill>
        <Pill active={view === "oss"} onClick={() => setView("oss")}>OSS / Gate</Pill>
        <Pill active={view === "layers"} onClick={() => setView("layers")}>5 Schichten</Pill>
        <Pill active={view === "migration"} onClick={() => setView("migration")}>Migration</Pill>
      </Row>

      {view === "overview" && <OverviewView />}
      {view === "traits" && <TraitsView />}
      {view === "chiptuner" && <ChiptunerView />}
      {view === "progress" && <ProgressView />}
      {view === "oss" && <OSSView />}
      {view === "layers" && <LayersView />}
      {view === "migration" && <MigrationView />}
    </Stack>
  );
}
