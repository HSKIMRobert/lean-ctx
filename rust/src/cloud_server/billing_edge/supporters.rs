//! Public supporters wall + supporter checkout.

#[allow(clippy::wildcard_imports)]
use super::*;

// ── Public supporters wall ────────────────────────────────────────────────────

/// How long a fetched supporters payload stays fresh before the next request
/// re-validates against the private plane. The wall changes rarely; 5 minutes
/// keeps it lively while shielding the plane from per-pageview traffic.
pub(super) const SUPPORTERS_CACHE_TTL: Duration = Duration::from_mins(5);
/// Plaintext clamp for a supporter's display name.
pub(super) const SUPPORTER_NAME_MAX: usize = 80;
/// Plaintext clamp for a supporter's optional message.
pub(super) const SUPPORTER_MESSAGE_MAX: usize = 140;
/// Clamp for short metadata strings (tier / currency / RFC 3339 timestamp).
const SUPPORTER_META_MAX: usize = 40;

/// The last sanitized supporters payload and when it was fetched. Process-wide
/// because the wall is global (not per-user), so a single slot suffices.
pub(super) type SupportersCacheSlot = Mutex<Option<(Instant, Value)>>;
static SUPPORTERS_CACHE: SupportersCacheSlot = Mutex::new(None);

/// The cached wall, if it was stored less than `SUPPORTERS_CACHE_TTL` before
/// `now`. `now` is injected so expiry is unit-testable without sleeping.
pub(super) fn supporters_cache_fresh(slot: &SupportersCacheSlot, now: Instant) -> Option<Value> {
    let guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    guard
        .as_ref()
        .filter(|(at, _)| now.duration_since(*at) < SUPPORTERS_CACHE_TTL)
        .map(|(_, v)| v.clone())
}

/// Store a freshly sanitized wall payload, restarting the TTL window at `now`.
pub(super) fn supporters_cache_store(slot: &SupportersCacheSlot, now: Instant, value: &Value) {
    *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some((now, value.clone()));
}

/// The last stored wall regardless of age — the stale fallback served when the
/// private plane is unreachable (a stale wall beats a broken one).
pub(super) fn supporters_cache_last(slot: &SupportersCacheSlot) -> Option<Value> {
    slot.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map(|(_, v)| v.clone())
}

/// Clamp supporter-provided free text to plain text (defense in depth — the
/// website renders via `textContent`, this protects every other consumer):
/// HTML tags are dropped, control characters and runs of whitespace collapse to
/// a single space, and the result is cut at `max` characters (char-boundary
/// safe for any UTF-8 input).
pub(super) fn sanitize_supporter_text(raw: &str, max: usize) -> String {
    // Pass 1: drop tag-shaped `<…>` runs, neutralize control characters. A `<`
    // only opens a tag when followed by a letter, `/` or `!`, so prose like
    // "i <3 rust" survives.
    let mut plain = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<'
            && matches!(chars.peek(), Some(n) if n.is_ascii_alphabetic() || *n == '/' || *n == '!')
        {
            for tag_char in chars.by_ref() {
                if tag_char == '>' {
                    break;
                }
            }
            plain.push(' ');
            continue;
        }
        plain.push(if c.is_control() { ' ' } else { c });
    }

    // Pass 2: collapse whitespace, trim, clamp to `max` characters.
    let mut out = String::with_capacity(plain.len().min(max * 4));
    let mut count = 0usize;
    let mut last_was_space = true; // swallows leading whitespace
    for c in plain.chars() {
        let c = if c.is_whitespace() { ' ' } else { c };
        if c == ' ' && last_was_space {
            continue;
        }
        last_was_space = c == ' ';
        out.push(c);
        count += 1;
        if count == max {
            break;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Rebuild the upstream supporters payload whitelist-style: only the documented
/// fields survive (anything else the plane might ever leak is dropped), every
/// free-text field is clamped to plain text, and `count` is recomputed instead
/// of trusted.
pub(super) fn sanitize_supporters_payload(raw: &Value) -> Value {
    let supporters: Vec<Value> = raw
        .get("supporters")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|entry| {
                    if !entry.is_object() {
                        return None;
                    }
                    let text = |field: &str, max: usize| {
                        sanitize_supporter_text(
                            entry.get(field).and_then(Value::as_str).unwrap_or(""),
                            max,
                        )
                    };
                    let message = text("message", SUPPORTER_MESSAGE_MAX);
                    Some(json!({
                        "name": text("name", SUPPORTER_NAME_MAX),
                        "message": if message.is_empty() { Value::Null } else { Value::String(message) },
                        "tier": text("tier", SUPPORTER_META_MAX),
                        "amount_cents": entry.get("amount_cents").and_then(Value::as_i64).unwrap_or(0).max(0),
                        "currency": text("currency", SUPPORTER_META_MAX),
                        "created_at": text("created_at", SUPPORTER_META_MAX),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();

    json!({ "count": supporters.len(), "supporters": supporters })
}

/// `GET /api/supporters` — the public supporters wall (no auth). Proxies the
/// private plane's read model with the shared internal key (which never reaches
/// the browser), sanitizes every supporter field to clamped plain text, and
/// serves from a 5-minute in-memory cache so page views don't hammer the plane.
///
/// Failure ladder:
/// - billing unconfigured ⇒ `200` with an empty wall (a standalone community
///   backend has no supporters read-model; the website still renders),
/// - upstream unreachable but a wall was fetched before ⇒ the stale copy,
/// - upstream unreachable and nothing cached ⇒ `503 {"error":"supporters_unavailable"}`.
pub(crate) async fn get_supporters(State(state): State<AppState>) -> Response {
    let (Some(base), Some(key)) = (
        state.cfg.billing_base_url.clone(),
        state.cfg.billing_internal_key.clone(),
    ) else {
        return Json(json!({ "supporters": [], "count": 0 })).into_response();
    };

    let now = Instant::now();
    if let Some(fresh) = supporters_cache_fresh(&SUPPORTERS_CACHE, now) {
        return Json(fresh).into_response();
    }

    let url = format!("{base}/api/billing/supporters");
    let body = tokio::task::spawn_blocking(move || {
        ureq::get(&url)
            .header("X-Internal-Key", &key)
            .call()
            .ok()?
            .into_body()
            .read_to_string()
            .ok()
    })
    .await
    .ok()
    .flatten();

    match body.and_then(|b| serde_json::from_str::<Value>(&b).ok()) {
        Some(raw) => {
            let clean = sanitize_supporters_payload(&raw);
            supporters_cache_store(&SUPPORTERS_CACHE, now, &clean);
            Json(clean).into_response()
        }
        None => match supporters_cache_last(&SUPPORTERS_CACHE) {
            Some(stale) => Json(stale).into_response(),
            None => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "supporters_unavailable" })),
            )
                .into_response(),
        },
    }
}

/// Request body for `POST /api/supporters/checkout`: the chosen monthly
/// contribution in USD minor units (cents).
#[derive(Deserialize)]
pub(crate) struct SupporterCheckoutBody {
    #[serde(default)]
    amount_cents: i64,
}

/// `POST /api/supporters/checkout` — start a no-account, custom-amount Supporter
/// subscription and return the hosted Stripe `url`. Public: supporting needs no
/// login. The amount is clamped here (defense in depth) and again on the private
/// plane; a 503/502 lets the website fall back to a fixed preset Payment Link.
pub(crate) async fn post_supporter_checkout(
    State(state): State<AppState>,
    Json(body): Json<SupporterCheckoutBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let amount = body.amount_cents.clamp(100, 100_000);
    Ok(Json(
        billing_post(
            &state.cfg,
            "/api/billing/supporters/checkout",
            json!({ "amount_cents": amount }),
        )
        .await?,
    ))
}
