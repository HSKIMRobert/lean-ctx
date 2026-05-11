//! U-shaped placement (primacy/recency): alternate placing ranked items at context head vs tail.

/// Sort by descending importance, then assign alternating fronts (`front`) vs backs (`back`).
pub fn reorder_for_attention(items: &mut [(String, f64)]) {
    if items.is_empty() {
        return;
    }
    let mut sorted: Vec<(String, f64)> = items.iter().map(|(s, r)| (s.clone(), *r)).collect();
    sorted.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    let n = sorted.len();
    let mut front: Vec<(String, f64)> = Vec::with_capacity(n.div_ceil(2));
    let mut back: Vec<(String, f64)> = Vec::with_capacity(n / 2);

    for (idx, pair) in sorted.into_iter().enumerate() {
        if idx % 2 == 0 {
            front.push(pair);
        } else {
            back.push(pair);
        }
    }

    let mut out = Vec::with_capacity(n);
    out.extend(front);
    out.extend(back.into_iter().rev());
    for (i, pair) in out.into_iter().enumerate() {
        items[i] = pair;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_stable() {
        let mut v: Vec<(String, f64)> = vec![];
        reorder_for_attention(&mut v);
        assert!(v.is_empty());
    }

    #[test]
    fn highest_front_second_back_next_near_front() {
        let mut v = vec![
            ("c".into(), 3.0),
            ("a".into(), 1.0),
            ("d".into(), 4.0),
            ("b".into(), 2.0),
        ];
        reorder_for_attention(&mut v);
        assert_eq!(v[0].0, "d");
        assert_eq!(v[v.len() - 1].0, "c");
        assert_eq!(v[1].0, "b");
        assert_eq!(v[v.len() - 2].0, "a");
    }

    #[test]
    fn ties_lexicographic_stable_middle_behavior() {
        let mut v = vec![
            ("z_last".into(), 1.0),
            ("a_first".into(), 1.0),
            ("m_mid".into(), 1.0),
        ];
        reorder_for_attention(&mut v);
        assert_eq!(v[0].0, "a_first");
        assert_eq!(v[v.len() - 1].0, "m_mid");
        assert_eq!(v[1].0, "z_last");
    }
}
