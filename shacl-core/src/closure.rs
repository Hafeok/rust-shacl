//! Generic least-fixpoint reachability — the provable core (§4.1, `REQ-PATH-7`).
//!
//! Backs three things (write & property-test once, §11.4): path `*`/`+` closure (`REQ-PATH-5/7`),
//! SHACL-subclass walking (`REQ-CLASS-2`), and recursion cycle detection (§9.1, ADR-002).
//!
//! Formal basis: given a `step` relation on a finite node set, the reflexive-transitive closure is
//! the least fixpoint of `R ↦ seed ∪ (R ∘ step)`. Since the node set is finite, the lattice is a
//! finite complete lattice and `step` is monotone, so Knaster–Tarski guarantees the fixpoint exists
//! and is reached in ≤ |V| frontier rounds. Termination on cyclic input is therefore guaranteed,
//! not best-effort.

use indexmap::IndexSet;
use std::hash::Hash;

/// Compute the set of nodes reachable from `start` by following `step` zero or more times
/// (reflexive-transitive closure). `start` itself is included.
///
/// `step(n)` returns the immediate successors of `n`. BFS frontier; visits each node once, so it
/// terminates even when `step` describes a cyclic relation (`REQ-PATH-7`). The result is an
/// [`IndexSet`] (insertion-ordered) because the term type is `Hash + Eq` but not `Ord`.
pub fn reachable_star<T, F, I>(start: T, mut step: F) -> IndexSet<T>
where
    T: Eq + Clone + Hash,
    F: FnMut(&T) -> I,
    I: IntoIterator<Item = T>,
{
    let mut seen = IndexSet::new();
    let mut frontier = vec![start.clone()];
    seen.insert(start);
    while let Some(n) = frontier.pop() {
        for next in step(&n) {
            if seen.insert(next.clone()) {
                frontier.push(next);
            }
        }
    }
    seen
}

/// One-or-more closure (transitive, non-reflexive): like [`reachable_star`] but `start` is included
/// only if it is reachable from itself via at least one step (`REQ-PATH-7`, `p+`).
pub fn reachable_plus<T, F, I>(start: T, mut step: F) -> IndexSet<T>
where
    T: Eq + Clone + Hash,
    F: FnMut(&T) -> I,
    I: IntoIterator<Item = T>,
{
    let mut seen = IndexSet::new();
    let mut frontier = Vec::new();
    // Seed with the *successors* of start, not start itself.
    for next in step(&start) {
        if seen.insert(next.clone()) {
            frontier.push(next);
        }
    }
    while let Some(n) = frontier.pop() {
        for next in step(&n) {
            if seen.insert(next.clone()) {
                frontier.push(next);
            }
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::{BTreeMap, BTreeSet};

    /// Normalize an insertion-ordered closure result to a `BTreeSet` so it can be compared against
    /// the naive oracle regardless of visitation order.
    fn sorted(s: IndexSet<u8>) -> BTreeSet<u8> {
        s.into_iter().collect()
    }

    /// Naive reference oracle (§4.1): reflexive-transitive closure by repeated relational
    /// composition until no change. Obviously correct, O(n^3)-ish, never shipped.
    fn naive_star(start: u8, edges: &BTreeMap<u8, Vec<u8>>) -> BTreeSet<u8> {
        let mut r: BTreeSet<u8> = BTreeSet::new();
        r.insert(start);
        loop {
            let mut grew = false;
            let snapshot: Vec<u8> = r.iter().copied().collect();
            for n in snapshot {
                if let Some(succ) = edges.get(&n) {
                    for &m in succ {
                        if r.insert(m) {
                            grew = true;
                        }
                    }
                }
            }
            if !grew {
                break;
            }
        }
        r
    }

    proptest! {
        /// (c) production == oracle, on random finite graphs incl. cycles. §4.1 REQ-PATH-9.
        #[test]
        fn star_matches_naive_oracle(
            edges in prop::collection::btree_map(
                0u8..12, prop::collection::vec(0u8..12, 0..6), 0..12),
            start in 0u8..12,
        ) {
            let step = |n: &u8| edges.get(n).cloned().unwrap_or_default();
            let got = sorted(reachable_star(start, step));
            let want = naive_star(start, &edges);
            prop_assert_eq!(got, want);
        }

        /// (a) idempotence: star of star adds nothing new. §4.1 REQ-PATH-9.
        #[test]
        fn star_is_idempotent(
            edges in prop::collection::btree_map(
                0u8..12, prop::collection::vec(0u8..12, 0..6), 0..12),
            start in 0u8..12,
        ) {
            let step = |n: &u8| edges.get(n).cloned().unwrap_or_default();
            let once = sorted(reachable_star(start, step));
            // Re-close from every member; union must equal `once`.
            let mut twice = BTreeSet::new();
            for &s in &once {
                let step2 = |n: &u8| edges.get(n).cloned().unwrap_or_default();
                twice.extend(reachable_star(s, step2));
            }
            prop_assert_eq!(once, twice);
        }

        /// (d) termination: reachable set never exceeds the universe of nodes. §4.1 REQ-PATH-9.
        #[test]
        fn star_bounded_by_universe(
            edges in prop::collection::btree_map(
                0u8..12, prop::collection::vec(0u8..12, 0..6), 0..12),
            start in 0u8..12,
        ) {
            let step = |n: &u8| edges.get(n).cloned().unwrap_or_default();
            let got = reachable_star(start, step);
            prop_assert!(got.len() <= 13); // 0..=12
        }
    }

    #[test]
    fn star_includes_start_even_with_no_edges() {
        let edges: BTreeMap<u8, Vec<u8>> = BTreeMap::new();
        let step = |n: &u8| edges.get(n).cloned().unwrap_or_default();
        let got = sorted(reachable_star(5u8, step));
        assert_eq!(got, BTreeSet::from([5]));
    }

    #[test]
    fn plus_excludes_start_unless_in_a_cycle() {
        // 1 -> 2 -> 3 (no cycle back to 1): plus(1) = {2,3}, star(1) = {1,2,3}
        let edges = BTreeMap::from([(1u8, vec![2]), (2u8, vec![3])]);
        let step = |n: &u8| edges.get(n).cloned().unwrap_or_default();
        assert_eq!(sorted(reachable_plus(1u8, step)), BTreeSet::from([2, 3]));
        let step2 = |n: &u8| edges.get(n).cloned().unwrap_or_default();
        assert_eq!(sorted(reachable_star(1u8, step2)), BTreeSet::from([1, 2, 3]));
    }

    #[test]
    fn plus_includes_start_when_cyclic() {
        // 1 -> 2 -> 1 : plus(1) includes 1 (reachable via a step). REQ-PATH-7.
        let edges = BTreeMap::from([(1u8, vec![2]), (2u8, vec![1])]);
        let step = |n: &u8| edges.get(n).cloned().unwrap_or_default();
        assert_eq!(sorted(reachable_plus(1u8, step)), BTreeSet::from([1, 2]));
    }
}
