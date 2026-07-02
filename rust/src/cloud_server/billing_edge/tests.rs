//! Tests for the billing edge (sync gate, entitlements cache, audit query).

use super::supporters::{
    SUPPORTER_MESSAGE_MAX, SUPPORTER_NAME_MAX, SUPPORTERS_CACHE_TTL, SupportersCacheSlot,
    sanitize_supporter_text, sanitize_supporters_payload, supporters_cache_fresh,
    supporters_cache_last, supporters_cache_store,
};
#[allow(clippy::wildcard_imports)]
use super::*;

/// A `Config` carrying only the two knobs the sync gate reads. `billing`
/// toggles whether a commercial plane is wired; `sync_open` is the operator
/// opt-out (`LEANCTX_CLOUD_SYNC_OPEN`).
fn cfg(billing: bool, sync_open: bool) -> Config {
    Config {
        bind_host: "127.0.0.1".into(),
        bind_port: 8088,
        public_base_url: String::new(),
        api_base_url: String::new(),
        database_url: String::new(),
        ip_hash_salt: String::new(),
        smtp_host: None,
        smtp_port: None,
        smtp_username: None,
        smtp_password: None,
        smtp_from: None,
        billing_base_url: billing.then(|| "https://billing.example".to_string()),
        billing_internal_key: billing.then(|| "internal-key".to_string()),
        sync_open,
    }
}

#[test]
fn gated_deployment_blocks_free_and_supporter_only() {
    // leanctx.com: billing wired, no operator opt-out → the gate is live.
    let gated = cfg(true, false);
    // Free and Supporter lack `cloud_sync` ⇒ denied (handler returns 402).
    assert!(!cloud_sync_allowed(&gated, Plan::Free));
    assert!(!cloud_sync_allowed(&gated, Plan::Supporter));
    // Pro and every superset grant `cloud_sync` ⇒ allowed.
    assert!(cloud_sync_allowed(&gated, Plan::Pro));
    assert!(cloud_sync_allowed(&gated, Plan::Team));
    assert!(cloud_sync_allowed(&gated, Plan::Enterprise));
}

#[test]
fn no_billing_plane_never_gates_sync() {
    // A self-hosted community backend without billing keeps sync fully usable
    // for every logged-in user (Local-Free Invariant) — even Free.
    let open = cfg(false, false);
    assert!(sync_is_open(&open));
    assert!(cloud_sync_allowed(&open, Plan::Free));
}

#[test]
fn operator_opt_out_opens_sync_even_with_billing() {
    // Self-host with billing wired but LEANCTX_CLOUD_SYNC_OPEN=1 → sync free
    // for everyone, regardless of plan.
    let opt_out = cfg(true, true);
    assert!(sync_is_open(&opt_out));
    assert!(cloud_sync_allowed(&opt_out, Plan::Free));
}

// ── Supporters wall: sanitization ─────────────────────────────────────────

#[test]
fn supporter_text_strips_html_and_control_chars() {
    let dirty = "Eve <script>alert('x')</script>\u{0007}\n<b>!</b>";
    assert_eq!(
        sanitize_supporter_text(dirty, SUPPORTER_NAME_MAX),
        "Eve alert('x') !"
    );
    // A bare `<` that doesn't open a tag is normal prose and survives.
    assert_eq!(
        sanitize_supporter_text("i <3 rust & you", SUPPORTER_MESSAGE_MAX),
        "i <3 rust & you"
    );
}

#[test]
fn supporter_text_clamps_length_on_char_boundaries() {
    // Multi-byte input must clamp by characters, not bytes (no panics, no
    // split code points). 90 'ä' → exactly 80 chars.
    let long_name = "ä".repeat(90);
    let clamped = sanitize_supporter_text(&long_name, SUPPORTER_NAME_MAX);
    assert_eq!(clamped.chars().count(), SUPPORTER_NAME_MAX);

    let long_message = "m".repeat(500);
    assert_eq!(
        sanitize_supporter_text(&long_message, SUPPORTER_MESSAGE_MAX).len(),
        SUPPORTER_MESSAGE_MAX
    );

    // Whitespace runs (incl. tabs/newlines) collapse and ends are trimmed.
    assert_eq!(
        sanitize_supporter_text("  a \t\t b\n\nc  ", SUPPORTER_NAME_MAX),
        "a b c"
    );
}

#[test]
fn supporters_payload_is_whitelisted_and_recounted() {
    let raw = json!({
        "supporters": [
            {
                "name": "<b>Ada</b>",
                "message": "",
                "tier": "Sponsor",
                "amount_cents": 2500,
                "currency": "usd",
                "created_at": "2026-05-01T10:00:00Z",
                "email": "leak@example.com"
            },
            "not-an-object"
        ],
        "count": 99
    });

    let clean = sanitize_supporters_payload(&raw);
    // Non-object entries are dropped and `count` is recomputed, not trusted.
    assert_eq!(clean["count"], 1);
    assert_eq!(clean["supporters"].as_array().map(Vec::len), Some(1));

    let s = &clean["supporters"][0];
    assert_eq!(s["name"], "Ada");
    // Empty message normalizes to null so clients can simply skip it.
    assert!(s["message"].is_null());
    assert_eq!(s["tier"], "Sponsor");
    assert_eq!(s["amount_cents"], 2500);
    assert_eq!(s["currency"], "usd");
    assert_eq!(s["created_at"], "2026-05-01T10:00:00Z");
    // Unknown upstream fields never pass the edge.
    assert!(s.get("email").is_none());
}

#[test]
fn supporters_payload_handles_malformed_upstream_shapes() {
    // No `supporters` array at all → an empty, well-formed wall.
    let clean = sanitize_supporters_payload(&json!({ "unexpected": true }));
    assert_eq!(clean["count"], 0);
    assert_eq!(clean["supporters"].as_array().map(Vec::len), Some(0));
}

// ── Supporters wall: cache ────────────────────────────────────────────────

#[test]
fn supporters_cache_hit_expiry_and_stale_fallback() {
    let slot: SupportersCacheSlot = Mutex::new(None);
    let t0 = Instant::now();

    // Empty cache: neither fresh nor stale.
    assert!(supporters_cache_fresh(&slot, t0).is_none());
    assert!(supporters_cache_last(&slot).is_none());

    let wall = json!({ "count": 1, "supporters": [{ "name": "Ada" }] });
    supporters_cache_store(&slot, t0, &wall);

    // Fresh within the TTL window (just before expiry).
    let just_before = (t0 + SUPPORTERS_CACHE_TTL)
        .checked_sub(Duration::from_secs(1))
        .unwrap();
    assert_eq!(
        supporters_cache_fresh(&slot, just_before),
        Some(wall.clone())
    );

    // At/after the TTL the entry no longer counts as fresh…
    assert!(supporters_cache_fresh(&slot, t0 + SUPPORTERS_CACHE_TTL).is_none());
    // …but stays available as the stale fallback for upstream outages.
    assert_eq!(supporters_cache_last(&slot), Some(wall.clone()));

    // Storing again restarts the TTL window.
    let t1 = t0 + SUPPORTERS_CACHE_TTL + Duration::from_secs(10);
    supporters_cache_store(&slot, t1, &wall);
    assert_eq!(supporters_cache_fresh(&slot, t1), Some(wall));
}

// ── Entitlements cache (GL #785) ──────────────────────────────────────────

#[test]
fn entitlements_cache_fresh_then_expiry_then_stale_fallback() {
    let slot: EntitlementsCacheSlot = Mutex::new(HashMap::new());
    let uid = Uuid::new_v4();
    let t0 = Instant::now();

    // Cold cache: neither fresh nor stale.
    assert!(entitlements_cache_fresh(&slot, uid, t0).is_none());
    assert!(entitlements_cache_any(&slot, uid).is_none());

    let pro = json!({ "plan": "pro", "entitlements": { "cloud_sync": true } });
    entitlements_cache_store(&slot, uid, t0, &pro);

    // Fresh just before the TTL window closes.
    let just_before = (t0 + ENTITLEMENTS_CACHE_TTL)
        .checked_sub(Duration::from_secs(1))
        .unwrap();
    assert_eq!(
        entitlements_cache_fresh(&slot, uid, just_before),
        Some(pro.clone())
    );

    // At/after the TTL it is no longer fresh…
    assert!(entitlements_cache_fresh(&slot, uid, t0 + ENTITLEMENTS_CACHE_TTL).is_none());
    // …but survives as the stale fallback served during a billing outage.
    assert_eq!(entitlements_cache_any(&slot, uid), Some(pro));
}

#[test]
fn entitlements_cache_stale_fallback_is_per_user() {
    let slot: EntitlementsCacheSlot = Mutex::new(HashMap::new());
    let seen = Uuid::new_v4();
    let never_seen = Uuid::new_v4();
    let t0 = Instant::now();

    entitlements_cache_store(&slot, seen, t0, &json!({ "plan": "pro" }));

    // A previously-seen payer keeps Pro during an outage; an account we have
    // never resolved has nothing to fall back to → caller degrades to Free.
    assert_eq!(
        entitlements_cache_any(&slot, seen),
        Some(json!({ "plan": "pro" }))
    );
    assert!(entitlements_cache_any(&slot, never_seen).is_none());
}

#[test]
fn prune_entitlements_cache_evicts_only_very_old_entries() {
    let mut map: HashMap<Uuid, CachedEntitlements> = HashMap::new();
    let t0 = Instant::now();
    let later = t0 + ENTITLEMENTS_STALE_RETAIN + Duration::from_secs(1);

    let old = Uuid::new_v4();
    let recent = Uuid::new_v4();
    map.insert(
        old,
        CachedEntitlements {
            at: t0,
            value: json!({ "plan": "team" }),
        },
    );
    map.insert(
        recent,
        CachedEntitlements {
            at: later,
            value: json!({ "plan": "pro" }),
        },
    );

    prune_entitlements_cache(&mut map, later);

    assert!(
        !map.contains_key(&old),
        "entries past the stale-retain window are dropped"
    );
    assert!(map.contains_key(&recent), "recent entries are kept");
}
