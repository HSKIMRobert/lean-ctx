//! Sleep-inspired consolidation for in-memory knowledge entries (NREM merge, REM prune, replay boost).
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

/// Knowledge unit subject to consolidation.
#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeEntry {
    pub key: String,
    pub content: String,
    pub access_count: u64,
    /// Unix timestamp (seconds) of last access.
    pub last_access: u64,
    /// Unix timestamp (seconds) when created.
    pub created_at: u64,
    pub importance: f64,
}

/// Jaccard similarity over whitespace-split tokens (case-folded).
pub fn token_jaccard(a: &str, b: &str) -> f64 {
    let sa: HashSet<String> = a.split_whitespace().map(str::to_lowercase).collect();
    let sb: HashSet<String> = b.split_whitespace().map(str::to_lowercase).collect();
    let inter = sa.intersection(&sb).count();
    let uni = sa.union(&sb).count();
    if uni == 0 {
        0.0
    } else {
        inter as f64 / uni as f64
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn days_since(ts: u64, now: u64) -> f64 {
    now.saturating_sub(ts) as f64 / 86400.0
}

const NREM_SIM_THRESHOLD: f64 = 0.8;
const REM_STALE_DAYS: f64 = 30.0;
const REM_MAX_IMPORTANCE: f64 = 0.35;
const REPLAY_RELATED_LOW: f64 = 0.12;
const REPLAY_RELATED_HIGH: f64 = 0.79;
const REPLAY_BOOST_SCALE: f64 = 0.02;

/// NREM: merge highly similar entries (keep highest-access body).
/// REM: drop stale + low-importance.
/// Replay: boost importance of pairwise related entries that are often accessed together (proxy).
pub fn consolidate(entries: &mut Vec<KnowledgeEntry>) {
    if entries.is_empty() {
        return;
    }
    nrem_merge(entries);
    let now = unix_now();
    rem_prune(entries, now);
    replay_boost(entries);
}

fn merge_two(dst: &mut KnowledgeEntry, src: &KnowledgeEntry) {
    let use_dst_body = dst.access_count > src.access_count
        || (dst.access_count == src.access_count && dst.importance >= src.importance);
    let total_access = dst.access_count.saturating_add(src.access_count);
    let la = dst.last_access.max(src.last_access);
    let ca = dst.created_at.min(src.created_at);
    let imp = dst.importance.max(src.importance);
    if use_dst_body {
        dst.access_count = total_access;
        dst.last_access = la;
        dst.created_at = ca;
        dst.importance = imp;
    } else {
        dst.key.clone_from(&src.key);
        dst.content.clone_from(&src.content);
        dst.access_count = total_access;
        dst.last_access = la;
        dst.created_at = ca;
        dst.importance = imp;
    }
}

fn nrem_merge(entries: &mut Vec<KnowledgeEntry>) {
    let mut out: Vec<KnowledgeEntry> = Vec::new();
    'outer: for e in entries.drain(..) {
        for slot in &mut out {
            if token_jaccard(&slot.content, &e.content) >= NREM_SIM_THRESHOLD {
                merge_two(slot, &e);
                continue 'outer;
            }
        }
        out.push(e);
    }
    *entries = out;
}

fn rem_prune(entries: &mut Vec<KnowledgeEntry>, now: u64) {
    entries.retain(|e| {
        let stale = days_since(e.last_access, now) >= REM_STALE_DAYS;
        !(stale && e.importance <= REM_MAX_IMPORTANCE)
    });
}

fn replay_boost(entries: &mut [KnowledgeEntry]) {
    let n = entries.len();
    if n < 2 {
        return;
    }
    let mut deltas = vec![0.0_f64; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let jac = token_jaccard(&entries[i].content, &entries[j].content);
            if !(REPLAY_RELATED_LOW..REPLAY_RELATED_HIGH).contains(&jac) {
                continue;
            }
            let co = ((entries[i].access_count as f64 + 1.0).ln()
                * (entries[j].access_count as f64 + 1.0).ln())
            .sqrt();
            let bump = REPLAY_BOOST_SCALE * co;
            deltas[i] += bump;
            deltas[j] += bump;
        }
    }
    for (e, d) in entries.iter_mut().zip(deltas) {
        e.importance += d;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts_days_ago(days: u64) -> u64 {
        unix_now().saturating_sub(days * 86400)
    }

    #[test]
    fn nrem_merges_similar_keeps_most_accessed_body() {
        let mut v = vec![
            KnowledgeEntry {
                key: "a".into(),
                content: "alpha beta gamma delta".into(),
                access_count: 2,
                last_access: unix_now(),
                created_at: 1,
                importance: 0.5,
            },
            KnowledgeEntry {
                key: "b".into(),
                content: "alpha beta gamma delta epsilon".into(),
                access_count: 10,
                last_access: unix_now(),
                created_at: 2,
                importance: 0.4,
            },
        ];
        consolidate(&mut v);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].content, "alpha beta gamma delta epsilon");
        assert_eq!(v[0].access_count, 12);
    }

    #[test]
    fn rem_drops_stale_low_importance() {
        let old = ts_days_ago(40);
        let mut v = vec![
            KnowledgeEntry {
                key: "keep".into(),
                content: "unique one".into(),
                access_count: 0,
                last_access: old,
                created_at: 0,
                importance: 0.9,
            },
            KnowledgeEntry {
                key: "gone".into(),
                content: "unique two".into(),
                access_count: 0,
                last_access: old,
                created_at: 0,
                importance: 0.2,
            },
        ];
        consolidate(&mut v);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].key, "keep");
    }

    #[test]
    fn replay_raises_importance_for_related_accessed_pairs() {
        let mut v = vec![
            KnowledgeEntry {
                key: "1".into(),
                content: "foo bar baz quux widget".into(),
                access_count: 100,
                last_access: unix_now(),
                created_at: 0,
                importance: 0.5,
            },
            KnowledgeEntry {
                key: "2".into(),
                content: "foo bar baz quux wobble".into(),
                access_count: 100,
                last_access: unix_now(),
                created_at: 0,
                importance: 0.5,
            },
            KnowledgeEntry {
                key: "3".into(),
                content: "totally different xyz".into(),
                access_count: 1,
                last_access: unix_now(),
                created_at: 0,
                importance: 0.5,
            },
        ];
        let unrelated_imp = v[2].importance;
        consolidate(&mut v);
        assert!(v[0].importance > 0.5 || v[1].importance > 0.5);
        assert!((v.iter().find(|e| e.key == "3").unwrap().importance - unrelated_imp).abs() < 1e-9);
    }
}
