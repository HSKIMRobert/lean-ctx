//! Org-policy gateway gate (enterprise#25) — model ceiling + hard budgets,
//! enforced in the forward path **only** under a signed, trusted,
//! `enforced = true` org policy ([`crate::core::policy::org`]).
//!
//! Two governance controls, both from the policy's new sections (Doc 08 §4.3):
//!
//! 1. **Model ceiling** (`[routing].allowed_models`) — a request whose
//!    requested model matches no allowlist pattern is refused with 403 before
//!    it leaves the gateway.
//! 2. **Hard budgets** (`[budgets]`) — measured spend per person/UTC-day and
//!    per project/UTC-month; a breached cap refuses further requests with 429
//!    until the window rolls over.
//!
//! Spend accounting feeds from the same choke-point as all metering
//! ([`super::usage_meter::record`]) and is seeded from the central usage
//! store when the gateway runs with Postgres, so budgets survive restarts and
//! cover multi-replica deployments to the seeding interval's precision.
//!
//! Design guarantees:
//! - **Local-free invariant:** without an installed + pinned + enforced org
//!   policy this module is a no-op — a solo user's traffic is never gated.
//! - **Fail-open on infrastructure:** seeding errors only degrade precision
//!   (in-process counting continues); they never block traffic.
//! - **O(1) per request:** the policy snapshot is cached with a short TTL;
//!   budget lookups are two hash-map reads.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::core::policy::{BudgetRules, RoutingPolicyRules};

/// How long a loaded org-policy snapshot stays valid before the gate re-reads
/// (and re-verifies) the installed artifact. Policy rollout latency, not a
/// hot-path cost: within the TTL every request uses the cached snapshot.
const SNAPSHOT_TTL: Duration = Duration::from_mins(1);

/// The governance subset of the active org policy the gate enforces.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GateRules {
    pub allowed_models: Vec<String>,
    pub forbid_downgrade_for: Vec<String>,
    pub max_cost_usd_per_person_per_day: Option<f64>,
    pub max_cost_usd_per_project_per_month: Option<f64>,
}

impl GateRules {
    fn from_policy(routing: &RoutingPolicyRules, budgets: &BudgetRules) -> Option<Self> {
        if routing.is_empty() && budgets.is_empty() {
            return None;
        }
        Some(Self {
            allowed_models: routing.allowed_models.clone(),
            forbid_downgrade_for: routing.forbid_downgrade_for.clone(),
            max_cost_usd_per_person_per_day: budgets.max_cost_usd_per_person_per_day,
            max_cost_usd_per_project_per_month: budgets.max_cost_usd_per_project_per_month,
        })
    }
}

struct CachedSnapshot {
    rules: Option<GateRules>,
    loaded_at: Instant,
}

static SNAPSHOT: RwLock<Option<CachedSnapshot>> = RwLock::new(None);

/// Test hook: pin the gate rules directly, bypassing the org-policy store.
/// Distinguishes "no override active" from "override pinned to no-rules".
#[cfg(test)]
#[derive(Clone, Default)]
enum TestOverride {
    #[default]
    Unset,
    Pinned(Option<GateRules>),
}

#[cfg(test)]
static TEST_OVERRIDE: Mutex<TestOverride> = Mutex::new(TestOverride::Unset);

/// The active governance rules, from cache or a fresh policy load.
/// `None` = no enforced org governance → the gate is a no-op.
#[must_use]
pub fn active_rules() -> Option<GateRules> {
    #[cfg(test)]
    if let TestOverride::Pinned(pinned) = TEST_OVERRIDE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
    {
        return pinned;
    }
    {
        let guard = SNAPSHOT
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(cached) = guard.as_ref()
            && cached.loaded_at.elapsed() < SNAPSHOT_TTL
        {
            return cached.rules.clone();
        }
    }
    let rules = crate::core::policy::org::active_resolved()
        .and_then(|p| GateRules::from_policy(&p.routing, &p.budgets));
    let mut guard = SNAPSHOT
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = Some(CachedSnapshot {
        rules: rules.clone(),
        loaded_at: Instant::now(),
    });
    rules
}

/// Pin (or clear) the gate rules for a test, bypassing disk + signatures.
#[cfg(test)]
pub fn test_set_rules(rules: Option<GateRules>) {
    *TEST_OVERRIDE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = TestOverride::Pinned(rules);
}

#[cfg(test)]
pub fn test_clear_rules() {
    *TEST_OVERRIDE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = TestOverride::Unset;
}

// ── Model ceiling ────────────────────────────────────────────────────────────

/// Glob-lite match: `*` matches any run of characters; everything else is
/// literal (models are flat names — no need for full glob semantics).
fn pattern_matches(pattern: &str, model: &str) -> bool {
    fn rec(p: &[u8], m: &[u8]) -> bool {
        match p.first() {
            None => m.is_empty(),
            Some(b'*') => {
                // Try every possible consumption length (bounded: model names
                // are short) — classic backtracking glob.
                (0..=m.len()).any(|k| rec(&p[1..], &m[k..]))
            }
            Some(&c) => m.first() == Some(&c) && rec(&p[1..], &m[1..]),
        }
    }
    rec(pattern.trim().as_bytes(), model.trim().as_bytes())
}

/// Whether the requested model passes the ceiling. An empty allowlist means
/// "no restriction".
#[must_use]
pub fn model_allowed(rules: &GateRules, model: &str) -> bool {
    rules.allowed_models.is_empty()
        || rules
            .allowed_models
            .iter()
            .any(|p| pattern_matches(p, model))
}

/// Whether the router must not downgrade this project's requests.
#[must_use]
pub fn downgrade_forbidden(rules: &GateRules, project: Option<&str>) -> bool {
    project.is_some_and(|p| rules.forbid_downgrade_for.iter().any(|f| f == p))
}

// ── Budget ledger ────────────────────────────────────────────────────────────

/// UTC day (`yyyymmdd`) and month (`yyyymm`) window keys.
fn window_keys_at(now: chrono::DateTime<chrono::Utc>) -> (u32, u32) {
    use chrono::Datelike;
    let day = now.year() as u32 * 10_000 + now.month() * 100 + now.day();
    let month = now.year() as u32 * 100 + now.month();
    (day, month)
}

fn window_keys() -> (u32, u32) {
    window_keys_at(chrono::Utc::now())
}

/// In-memory measured-spend accumulators for the two budget windows.
///
/// `baseline` holds sums seeded from the central usage store (authoritative
/// across restarts/replicas); `live` accumulates events recorded by *this*
/// process since the last seed. A seed replaces the baseline and clears the
/// live delta — the store query already contains those events.
#[derive(Default)]
struct BudgetLedger {
    day_key: u32,
    month_key: u32,
    baseline_person_day: HashMap<String, f64>,
    live_person_day: HashMap<String, f64>,
    baseline_project_month: HashMap<String, f64>,
    live_project_month: HashMap<String, f64>,
}

impl BudgetLedger {
    /// Drop accumulators whose window rolled over.
    fn roll(&mut self, day: u32, month: u32) {
        if self.day_key != day {
            self.day_key = day;
            self.baseline_person_day.clear();
            self.live_person_day.clear();
        }
        if self.month_key != month {
            self.month_key = month;
            self.baseline_project_month.clear();
            self.live_project_month.clear();
        }
    }

    fn add(&mut self, person: Option<&str>, project: Option<&str>, cost_usd: f64) {
        let (day, month) = window_keys();
        self.roll(day, month);
        if let Some(p) = person {
            *self.live_person_day.entry(p.to_string()).or_default() += cost_usd;
        }
        if let Some(p) = project {
            *self.live_project_month.entry(p.to_string()).or_default() += cost_usd;
        }
    }

    fn person_day_spend(&self, person: &str) -> f64 {
        self.baseline_person_day.get(person).copied().unwrap_or(0.0)
            + self.live_person_day.get(person).copied().unwrap_or(0.0)
    }

    fn project_month_spend(&self, project: &str) -> f64 {
        self.baseline_project_month
            .get(project)
            .copied()
            .unwrap_or(0.0)
            + self.live_project_month.get(project).copied().unwrap_or(0.0)
    }
}

fn ledger() -> &'static Mutex<BudgetLedger> {
    static LEDGER: OnceLock<Mutex<BudgetLedger>> = OnceLock::new();
    LEDGER.get_or_init(|| Mutex::new(BudgetLedger::default()))
}

/// Records one measured turn's cost against the budget windows. Called from
/// the metering choke-point; cheap (two hash-map bumps) and never blocking.
pub fn record_spend(person: Option<&str>, project: Option<&str>, cost_usd: f64) {
    if cost_usd <= 0.0 || (person.is_none() && project.is_none()) {
        return;
    }
    ledger()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .add(person, project, cost_usd);
}

/// Replaces the seeded baselines with fresh sums from the central usage store
/// (gateway-server mode). The live deltas reset — the store query already
/// includes everything this process pushed through the usage sink.
pub fn seed_from_store(person_day: HashMap<String, f64>, project_month: HashMap<String, f64>) {
    let (day, month) = window_keys();
    let mut guard = ledger()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.roll(day, month);
    guard.baseline_person_day = person_day;
    guard.live_person_day.clear();
    guard.baseline_project_month = project_month;
    guard.live_project_month.clear();
}

#[cfg(test)]
fn test_reset_ledger() {
    let mut guard = ledger()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = BudgetLedger::default();
}

// ── Enforcement ──────────────────────────────────────────────────────────────

/// Why the gate refused a request.
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
    ModelNotAllowed {
        model: String,
    },
    PersonBudgetExceeded {
        person: String,
        cap_usd: f64,
        spent_usd: f64,
    },
    ProjectBudgetExceeded {
        project: String,
        cap_usd: f64,
        spent_usd: f64,
    },
}

/// Blocked-request counters for `/metrics` (enterprise#34).
static BLOCKED_MODEL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static BLOCKED_BUDGET: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// (model-ceiling blocks, budget blocks) since process start.
#[must_use]
pub fn blocked_counters() -> (u64, u64) {
    (
        BLOCKED_MODEL.load(std::sync::atomic::Ordering::Relaxed),
        BLOCKED_BUDGET.load(std::sync::atomic::Ordering::Relaxed),
    )
}

/// The full gate: model ceiling, then budgets. `Ok(())` forwards; a refusal
/// carries everything needed to render the wire-shape error.
pub fn enforce(
    rules: &GateRules,
    requested_model: Option<&str>,
    tags: &super::gateway_identity::GatewayTags,
) -> Result<(), Refusal> {
    if let Some(model) = requested_model
        && !model_allowed(rules, model)
    {
        BLOCKED_MODEL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return Err(Refusal::ModelNotAllowed {
            model: model.to_string(),
        });
    }

    let guard = ledger()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let (Some(cap), Some(person)) = (
        rules.max_cost_usd_per_person_per_day,
        tags.person.as_deref(),
    ) {
        let spent = guard.person_day_spend(person);
        if spent >= cap {
            BLOCKED_BUDGET.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Err(Refusal::PersonBudgetExceeded {
                person: person.to_string(),
                cap_usd: cap,
                spent_usd: spent,
            });
        }
    }
    if let (Some(cap), Some(project)) = (
        rules.max_cost_usd_per_project_per_month,
        tags.project.as_deref(),
    ) {
        let spent = guard.project_month_spend(project);
        if spent >= cap {
            BLOCKED_BUDGET.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Err(Refusal::ProjectBudgetExceeded {
                project: project.to_string(),
                cap_usd: cap,
                spent_usd: spent,
            });
        }
    }
    Ok(())
}

/// Renders a refusal as the wire-shape error the client's SDK understands.
/// Model blocks → 403, budget blocks → 429 with `Retry-After`.
#[must_use]
pub fn refusal_response(refusal: &Refusal, provider_label: &str) -> Response {
    let openai_shape = matches!(provider_label, "OpenAI" | "ChatGPT");
    let (status, code, message) = match refusal {
        Refusal::ModelNotAllowed { model } => (
            StatusCode::FORBIDDEN,
            "org_policy_model_blocked",
            format!(
                "model '{model}' is not allowed by your organization's AI gateway policy — \
                 choose an approved model or contact your gateway admin"
            ),
        ),
        Refusal::PersonBudgetExceeded {
            person,
            cap_usd,
            spent_usd,
        } => (
            StatusCode::TOO_MANY_REQUESTS,
            "org_budget_exceeded",
            format!(
                "daily AI budget exhausted for '{person}' \
                 (${spent_usd:.2} of ${cap_usd:.2} spent) — resets at midnight UTC"
            ),
        ),
        Refusal::ProjectBudgetExceeded {
            project,
            cap_usd,
            spent_usd,
        } => (
            StatusCode::TOO_MANY_REQUESTS,
            "org_budget_exceeded",
            format!(
                "monthly AI budget exhausted for project '{project}' \
                 (${spent_usd:.2} of ${cap_usd:.2} spent) — resets on the 1st (UTC)"
            ),
        ),
    };

    let body = if openai_shape {
        serde_json::json!({
            "error": {
                "message": message,
                "type": if status == StatusCode::FORBIDDEN {
                    "invalid_request_error"
                } else {
                    "insufficient_quota"
                },
                "code": code,
            }
        })
    } else {
        // Anthropic error envelope (also what Gemini SDKs tolerate for
        // non-2xx JSON: they surface `message`).
        serde_json::json!({
            "type": "error",
            "error": {
                "type": if status == StatusCode::FORBIDDEN {
                    "permission_error"
                } else {
                    "rate_limit_error"
                },
                "message": message,
            }
        })
    };

    let mut response = (status, axum::Json(body)).into_response();
    if status == StatusCode::TOO_MANY_REQUESTS {
        // Coarse, honest hint: budgets roll at window boundaries, not in 60 s;
        // the header mainly stops naive SDK hot-retry loops.
        response.headers_mut().insert(
            axum::http::header::RETRY_AFTER,
            "3600".parse().expect("static"),
        );
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::gateway_identity::GatewayTags;

    fn tags(person: &str, project: &str) -> GatewayTags {
        GatewayTags {
            person: Some(person.to_string()),
            team: None,
            project: Some(project.to_string()),
        }
    }

    fn rules() -> GateRules {
        GateRules {
            allowed_models: vec!["claude-*".into(), "gpt-4o-mini".into()],
            forbid_downgrade_for: vec!["prod".into()],
            max_cost_usd_per_person_per_day: Some(50.0),
            max_cost_usd_per_project_per_month: Some(1000.0),
        }
    }

    #[test]
    fn glob_lite_matches_prefix_and_exact() {
        assert!(pattern_matches("claude-*", "claude-sonnet-4-5"));
        assert!(pattern_matches("gpt-4o-mini", "gpt-4o-mini"));
        assert!(pattern_matches("*", "anything"));
        assert!(!pattern_matches("claude-*", "gpt-5.2"));
        assert!(!pattern_matches("gpt-4o-mini", "gpt-4o"));
        assert!(pattern_matches("*sonnet*", "claude-sonnet-4-5"));
    }

    #[test]
    fn empty_allowlist_means_no_restriction() {
        let r = GateRules::default();
        assert!(model_allowed(&r, "any-model"));
    }

    #[test]
    #[serial_test::serial(policy_gate_ledger)]
    fn model_ceiling_blocks_unlisted_model() {
        test_reset_ledger();
        let err = enforce(&rules(), Some("o3-pro"), &tags("a", "p")).unwrap_err();
        assert_eq!(
            err,
            Refusal::ModelNotAllowed {
                model: "o3-pro".into()
            }
        );
        // Allowed models pass.
        assert!(enforce(&rules(), Some("claude-haiku-4-5"), &tags("a", "p")).is_ok());
    }

    #[test]
    #[serial_test::serial(policy_gate_ledger)]
    fn person_day_budget_blocks_after_cap() {
        test_reset_ledger();
        record_spend(Some("mara"), Some("web"), 49.0);
        assert!(enforce(&rules(), Some("claude-x"), &tags("mara", "web")).is_ok());
        record_spend(Some("mara"), Some("web"), 2.0);
        let err = enforce(&rules(), Some("claude-x"), &tags("mara", "web")).unwrap_err();
        match err {
            Refusal::PersonBudgetExceeded {
                person,
                cap_usd,
                spent_usd,
            } => {
                assert_eq!(person, "mara");
                assert!((cap_usd - 50.0).abs() < f64::EPSILON);
                assert!(spent_usd >= 51.0 - 1e-9);
            }
            other => panic!("expected person budget refusal, got {other:?}"),
        }
        test_reset_ledger();
    }

    #[test]
    #[serial_test::serial(policy_gate_ledger)]
    fn project_month_budget_blocks_after_cap() {
        test_reset_ledger();
        record_spend(Some("a"), Some("ml-pipeline"), 600.0);
        record_spend(Some("b"), Some("ml-pipeline"), 500.0);
        let err = enforce(&rules(), Some("claude-x"), &tags("c", "ml-pipeline")).unwrap_err();
        assert!(matches!(err, Refusal::ProjectBudgetExceeded { .. }));
        // Another project is unaffected.
        assert!(enforce(&rules(), Some("claude-x"), &tags("c", "other")).is_ok());
        test_reset_ledger();
    }

    #[test]
    #[serial_test::serial(policy_gate_ledger)]
    fn seeding_replaces_baseline_and_clears_live() {
        test_reset_ledger();
        record_spend(Some("mara"), Some("web"), 10.0);
        seed_from_store(HashMap::from([("mara".to_string(), 49.5)]), HashMap::new());
        // 49.5 seeded (live 10.0 discarded — the store already contains it).
        assert!(enforce(&rules(), Some("claude-x"), &tags("mara", "web")).is_ok());
        record_spend(Some("mara"), Some("web"), 0.6);
        assert!(enforce(&rules(), Some("claude-x"), &tags("mara", "web")).is_err());
        test_reset_ledger();
    }

    #[test]
    fn windows_roll_over() {
        let mut l = BudgetLedger::default();
        let (d1, m1) = (20_260_701, 202_607);
        l.roll(d1, m1);
        l.live_person_day.insert("a".into(), 100.0);
        l.live_project_month.insert("p".into(), 100.0);
        // Next day, same month: person resets, project persists.
        l.roll(20_260_702, m1);
        assert_eq!(l.person_day_spend("a"), 0.0);
        assert_eq!(l.project_month_spend("p"), 100.0);
        // Next month: project resets too.
        l.roll(20_260_801, 202_608);
        assert_eq!(l.project_month_spend("p"), 0.0);
    }

    #[test]
    #[serial_test::serial(policy_gate_ledger)]
    fn anonymous_requests_pass_budget_checks() {
        test_reset_ledger();
        // No tags → no person/project to attribute → budgets cannot apply.
        let err = enforce(&rules(), Some("claude-x"), &GatewayTags::default());
        assert!(err.is_ok());
    }

    #[test]
    fn downgrade_exemption_matches_project() {
        let r = rules();
        assert!(downgrade_forbidden(&r, Some("prod")));
        assert!(!downgrade_forbidden(&r, Some("web")));
        assert!(!downgrade_forbidden(&r, None));
    }

    #[test]
    fn refusal_bodies_match_wire_shape() {
        let model_block = Refusal::ModelNotAllowed { model: "o3".into() };
        let resp = refusal_response(&model_block, "Anthropic");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let budget_block = Refusal::PersonBudgetExceeded {
            person: "a".into(),
            cap_usd: 50.0,
            spent_usd: 51.0,
        };
        let resp = refusal_response(&budget_block, "OpenAI");
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(resp.headers().contains_key(axum::http::header::RETRY_AFTER));
    }
}
