//! Helpers shared by the conversions.

use crate::automaton::{Point, StateId, Transition};
use crate::fa::{Fa, FaLabel, has_multi_char_labels, split_multi_char_labels};

/// Deterministic position of the `index`-th state created by a conversion.
pub(super) fn grid_point(index: usize) -> Point {
    Point {
        x: 60.0 + 140.0 * (index % 5) as f64,
        y: 60.0 + 120.0 * (index / 5) as f64,
    }
}

/// Copy where every transition reads one character or nothing: multi-character labels are
/// chained through new states and `[x-y]` ranges become one transition per character.
pub(super) fn normalize(fa: &Fa) -> Fa {
    let mut out = if has_multi_char_labels(fa) {
        split_multi_char_labels(fa)
    } else {
        fa.clone()
    };
    for t in out.transitions().to_vec() {
        let Some((lo, hi)) = t.label.range() else {
            continue;
        };
        out.remove_transition(&t);
        for c in (lo as u32..=hi as u32).filter_map(char::from_u32) {
            out.add_transition(Transition {
                from: t.from,
                to: t.to,
                label: FaLabel(c.to_string()),
                control: t.control,
            });
        }
    }
    out
}

/// "0,1,3"
pub(super) fn join_ids(ids: &[StateId]) -> String {
    ids.iter()
        .map(StateId::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
