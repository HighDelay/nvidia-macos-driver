use std::collections::{BTreeMap, BTreeSet, VecDeque};

type Cut = BTreeSet<(Label, Label)>;

pub(super) type Label = u32;

pub(super) struct Graph {
    pub(super) entry: Label,
    pub(super) successors: BTreeMap<Label, Vec<Label>>,
    pub(super) predecessors: BTreeMap<Label, Vec<Label>>,
}

impl Graph {
    pub(super) fn new(entry: Label, successors: BTreeMap<Label, Vec<Label>>) -> Self {
        let mut predecessors: BTreeMap<Label, Vec<Label>> = BTreeMap::new();
        for label in successors.keys() {
            predecessors.entry(*label).or_default();
        }
        for (label, targets) in &successors {
            for target in targets {
                let list = predecessors.entry(*target).or_default();
                if !list.contains(label) {
                    list.push(*label);
                }
            }
        }
        Self {
            entry,
            successors,
            predecessors,
        }
    }

    pub(super) fn succ(&self, label: Label) -> &[Label] {
        self.successors.get(&label).map_or(&[], Vec::as_slice)
    }

    fn pred(&self, label: Label) -> &[Label] {
        self.predecessors.get(&label).map_or(&[], Vec::as_slice)
    }

    fn reachable_within(
        &self,
        label: Label,
        within: &BTreeSet<Label>,
        cut: &Cut,
    ) -> BTreeSet<Label> {
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::new();
        if within.contains(&label) {
            seen.insert(label);
            queue.push_back(label);
        }
        while let Some(current) = queue.pop_front() {
            for target in self.succ(current) {
                if within.contains(target)
                    && !cut.contains(&(current, *target))
                    && seen.insert(*target)
                {
                    queue.push_back(*target);
                }
            }
        }
        seen
    }

    pub(super) fn reachable(&self) -> BTreeSet<Label> {
        let all = self.successors.keys().copied().collect::<BTreeSet<_>>();
        self.reachable_within(self.entry, &all, &Cut::new())
    }

    pub(super) fn is_reducible(&self) -> bool {
        let dominators = self.dominators();
        let mut state: BTreeMap<Label, u8> = BTreeMap::new();
        let mut stack = vec![(self.entry, 0usize)];
        state.insert(self.entry, 1);
        while let Some((node, index)) = stack.pop() {
            let targets = self.succ(node);
            if index < targets.len() {
                stack.push((node, index + 1));
                let target = targets[index];
                match state.get(&target).copied().unwrap_or(0) {
                    0 => {
                        state.insert(target, 1);
                        stack.push((target, 0));
                    }
                    1 if !dominators.dominates(target, node) => {
                        return false;
                    }
                    _ => {}
                }
            } else {
                state.insert(node, 2);
            }
        }
        true
    }

    fn dominators(&self) -> Dominators {
        let index = self
            .successors
            .keys()
            .copied()
            .enumerate()
            .map(|(index, label)| (label, index))
            .collect::<BTreeMap<Label, usize>>();
        let Some(&entry) = index.get(&self.entry) else {
            return Dominators::default();
        };
        let swap = |slot: usize| match slot {
            0 => entry,
            slot if slot == entry => 0,
            slot => slot,
        };
        let mut successors = vec![Vec::new(); index.len()];
        for (label, targets) in &self.successors {
            successors[swap(index[label])] = targets
                .iter()
                .filter_map(|target| index.get(target).map(|target| swap(*target)))
                .collect();
        }
        let predecessors = crate::dominators::build_predecessors(&successors);
        let (_, intervals, _) = crate::dominators::dominance(&successors, &predecessors);
        Dominators {
            block: index
                .into_iter()
                .map(|(label, slot)| (label, swap(slot)))
                .collect(),
            intervals,
        }
    }
}

#[derive(Default)]
pub(super) struct Dominators {
    block: BTreeMap<Label, usize>,
    intervals: Vec<Option<(usize, usize)>>,
}

impl Dominators {
    pub(super) fn dominates(&self, ancestor: Label, node: Label) -> bool {
        let (Some(&ancestor), Some(&node)) = (self.block.get(&ancestor), self.block.get(&node))
        else {
            return false;
        };
        crate::dominators::dominates_interval(&self.intervals, ancestor, node)
    }

    #[cfg(test)]
    pub(super) fn reached(&self) -> usize {
        self.intervals.iter().filter(|slot| slot.is_some()).count()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Shape {
    Simple {
        id: usize,
        label: Label,
        next: Option<Box<Shape>>,
    },
    Loop {
        id: usize,
        entries: BTreeSet<Label>,
        inner: Box<Shape>,
        next: Option<Box<Shape>>,
    },
    Multiple {
        id: usize,
        entries: BTreeSet<Label>,
        handled: Vec<(Label, Shape)>,
        next: Option<Box<Shape>>,
    },
}

impl Shape {
    pub(super) fn id(&self) -> usize {
        match self {
            Shape::Simple { id, .. } | Shape::Loop { id, .. } | Shape::Multiple { id, .. } => *id,
        }
    }

    pub(super) fn entry_labels(&self) -> BTreeSet<Label> {
        match self {
            Shape::Simple { label, .. } => BTreeSet::from([*label]),
            Shape::Loop { entries, .. } | Shape::Multiple { entries, .. } => entries.clone(),
        }
    }
}

pub(super) fn calculate(graph: &Graph) -> Option<Shape> {
    let mut counter = 0usize;
    let blocks = graph.reachable();
    let entries = BTreeSet::from([graph.entry]);
    let mut cut = Cut::new();
    calculate_region(graph, blocks, entries, &mut cut, &mut counter)
}

fn calculate_region(
    graph: &Graph,
    mut blocks: BTreeSet<Label>,
    entries: BTreeSet<Label>,
    cut: &mut Cut,
    counter: &mut usize,
) -> Option<Shape> {
    let entries = entries
        .into_iter()
        .filter(|entry| blocks.contains(entry))
        .collect::<BTreeSet<_>>();
    if entries.is_empty() || blocks.is_empty() {
        return None;
    }
    if entries.len() == 1 {
        let label = *entries.iter().next()?;
        let reentered = graph.pred(label).iter().any(|predecessor| {
            blocks.contains(predecessor) && !cut.contains(&(*predecessor, label))
        });
        if !reentered {
            let id = fresh(counter);
            blocks.remove(&label);
            let next_entries = graph
                .succ(label)
                .iter()
                .copied()
                .filter(|target| blocks.contains(target) && !cut.contains(&(label, *target)))
                .collect::<BTreeSet<_>>();
            let next = calculate_region(graph, blocks, next_entries, cut, counter);
            return Some(Shape::Simple {
                id,
                label,
                next: next.map(Box::new),
            });
        }
        return Some(make_loop(graph, blocks, entries, cut, counter));
    }
    match independent_groups(graph, &blocks, &entries, cut) {
        Some(groups) => Some(make_multiple(graph, blocks, entries, groups, cut, counter)),
        None => Some(make_loop(graph, blocks, entries, cut, counter)),
    }
}

fn make_loop(
    graph: &Graph,
    mut blocks: BTreeSet<Label>,
    entries: BTreeSet<Label>,
    cut: &mut Cut,
    counter: &mut usize,
) -> Shape {
    let id = fresh(counter);
    let mut inner = BTreeSet::new();
    let mut queue = entries.iter().copied().collect::<VecDeque<_>>();
    while let Some(current) = queue.pop_front() {
        if !blocks.contains(&current) || !inner.insert(current) {
            continue;
        }
        for predecessor in graph.pred(current) {
            if blocks.contains(predecessor)
                && !cut.contains(&(*predecessor, current))
                && !inner.contains(predecessor)
            {
                queue.push_back(*predecessor);
            }
        }
    }
    blocks.retain(|label| !inner.contains(label));
    let next_entries = inner
        .iter()
        .flat_map(|label| {
            graph
                .succ(*label)
                .iter()
                .map(move |target| (*label, *target))
        })
        .filter(|edge| blocks.contains(&edge.1) && !cut.contains(edge))
        .map(|edge| edge.1)
        .collect::<BTreeSet<_>>();
    let back_edges = inner
        .iter()
        .flat_map(|label| {
            entries
                .iter()
                .filter(|entry| graph.succ(*label).contains(entry))
                .map(move |entry| (*label, *entry))
        })
        .collect::<Vec<_>>();
    let restore = back_edges
        .iter()
        .filter(|edge| cut.insert(**edge))
        .copied()
        .collect::<Vec<_>>();
    let body = calculate_region(graph, inner, entries.clone(), cut, counter);
    for edge in restore {
        cut.remove(&edge);
    }
    let next = calculate_region(graph, blocks, next_entries, cut, counter);
    let inner = body.unwrap_or_else(|| Shape::Multiple {
        id: fresh(counter),
        entries: entries.clone(),
        handled: Vec::new(),
        next: None,
    });
    Shape::Loop {
        id,
        entries,
        inner: Box::new(inner),
        next: next.map(Box::new),
    }
}

fn make_multiple(
    graph: &Graph,
    mut blocks: BTreeSet<Label>,
    entries: BTreeSet<Label>,
    groups: BTreeMap<Label, BTreeSet<Label>>,
    cut: &mut Cut,
    counter: &mut usize,
) -> Shape {
    let id = fresh(counter);
    let owned = groups
        .values()
        .flat_map(|group| group.iter().copied())
        .collect::<BTreeSet<_>>();
    blocks.retain(|label| !owned.contains(label));
    let mut handled = Vec::new();
    let mut next_entries = entries
        .iter()
        .copied()
        .filter(|entry| !groups.contains_key(entry))
        .collect::<BTreeSet<_>>();
    for (entry, group) in &groups {
        next_entries.extend(
            group
                .iter()
                .flat_map(|label| {
                    graph
                        .succ(*label)
                        .iter()
                        .map(move |target| (*label, *target))
                })
                .filter(|edge| blocks.contains(&edge.1) && !cut.contains(edge))
                .map(|edge| edge.1),
        );
        if let Some(shape) =
            calculate_region(graph, group.clone(), BTreeSet::from([*entry]), cut, counter)
        {
            handled.push((*entry, shape));
        }
    }
    let next = calculate_region(graph, blocks, next_entries, cut, counter);
    Shape::Multiple {
        id,
        entries,
        handled,
        next: next.map(Box::new),
    }
}

fn independent_groups(
    graph: &Graph,
    blocks: &BTreeSet<Label>,
    entries: &BTreeSet<Label>,
    cut: &Cut,
) -> Option<BTreeMap<Label, BTreeSet<Label>>> {
    let reach = entries
        .iter()
        .map(|entry| (*entry, graph.reachable_within(*entry, blocks, cut)))
        .collect::<BTreeMap<_, _>>();
    let mut groups = BTreeMap::new();
    for entry in entries {
        let Some(own) = reach.get(entry) else {
            continue;
        };
        let exclusive = own
            .iter()
            .copied()
            .filter(|label| {
                if entries.contains(label) && label != entry {
                    return false;
                }
                reach
                    .iter()
                    .all(|(other, seen)| other == entry || !seen.contains(label))
            })
            .collect::<BTreeSet<_>>();
        if !exclusive.is_empty() {
            groups.insert(*entry, exclusive);
        }
    }
    (!groups.is_empty()).then_some(groups)
}

fn fresh(counter: &mut usize) -> usize {
    let id = *counter;
    *counter += 1;
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(entry: Label, edges: &[(Label, &[Label])]) -> Graph {
        Graph::new(
            entry,
            edges
                .iter()
                .map(|(label, targets)| (*label, targets.to_vec()))
                .collect(),
        )
    }

    fn rendered(shape: &Shape, out: &mut Vec<Label>) {
        match shape {
            Shape::Simple { label, next, .. } => {
                out.push(*label);
                if let Some(next) = next {
                    rendered(next, out);
                }
            }
            Shape::Loop { inner, next, .. } => {
                rendered(inner, out);
                if let Some(next) = next {
                    rendered(next, out);
                }
            }
            Shape::Multiple { handled, next, .. } => {
                for (_, arm) in handled {
                    rendered(arm, out);
                }
                if let Some(next) = next {
                    rendered(next, out);
                }
            }
        }
    }

    #[test]
    fn straight_line_is_a_chain_of_simple_shapes() {
        let g = graph(1, &[(1, &[2]), (2, &[3]), (3, &[])]);
        let shape = calculate(&g).expect("shape");
        let mut order = Vec::new();
        rendered(&shape, &mut order);
        assert_eq!(order, vec![1, 2, 3]);
        assert!(matches!(shape, Shape::Simple { label: 1, .. }));
    }

    #[test]
    fn diamond_becomes_a_multiple_with_two_arms() {
        let g = graph(1, &[(1, &[2, 3]), (2, &[4]), (3, &[4]), (4, &[])]);
        let shape = calculate(&g).expect("shape");
        let Shape::Simple { label: 1, next, .. } = shape else {
            panic!("expected the header to be simple");
        };
        let next = next.expect("diamond has a continuation");
        let Shape::Multiple { handled, next, .. } = *next else {
            panic!("expected a dispatch over the two arms");
        };
        assert_eq!(
            handled.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert!(matches!(
            next.as_deref(),
            Some(Shape::Simple { label: 4, .. })
        ));
    }

    #[test]
    fn if_without_else_puts_the_single_arm_in_the_multiple() {
        let g = graph(1, &[(1, &[2, 3]), (2, &[3]), (3, &[])]);
        let shape = calculate(&g).expect("shape");
        let Shape::Simple { next, .. } = shape else {
            panic!("expected simple header");
        };
        let Some(box_next) = next else {
            panic!("expected a continuation")
        };
        let Shape::Multiple { handled, next, .. } = *box_next else {
            panic!("expected a dispatch");
        };
        assert_eq!(
            handled.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
            vec![2]
        );
        assert!(matches!(
            next.as_deref(),
            Some(Shape::Simple { label: 3, .. })
        ));
    }

    #[test]
    fn self_loop_becomes_a_loop_shape() {
        let g = graph(1, &[(1, &[2]), (2, &[2, 3]), (3, &[])]);
        let shape = calculate(&g).expect("shape");
        let Shape::Simple { label: 1, next, .. } = shape else {
            panic!("expected simple header");
        };
        let Shape::Loop { entries, next, .. } = *next.expect("loop follows") else {
            panic!("expected a loop");
        };
        assert_eq!(entries, BTreeSet::from([2]));
        assert!(matches!(
            next.as_deref(),
            Some(Shape::Simple { label: 3, .. })
        ));
    }

    #[test]
    fn nested_loops_nest_in_the_shape_tree() {
        let g = graph(
            1,
            &[(1, &[2]), (2, &[3]), (3, &[3, 4]), (4, &[2, 5]), (5, &[])],
        );
        let shape = calculate(&g).expect("shape");
        let Shape::Simple { next, .. } = shape else {
            panic!("expected simple header");
        };
        let Shape::Loop { inner, next, .. } = *next.expect("outer loop") else {
            panic!("expected the outer loop");
        };
        assert!(matches!(
            next.as_deref(),
            Some(Shape::Simple { label: 5, .. })
        ));
        let mut order = Vec::new();
        rendered(&inner, &mut order);
        assert_eq!(order, vec![2, 3, 4]);
    }

    #[test]
    fn every_reachable_block_is_rendered_exactly_once() {
        let g = graph(
            1,
            &[
                (1, &[2, 3]),
                (2, &[4]),
                (3, &[4, 5]),
                (4, &[6]),
                (5, &[6]),
                (6, &[7, 2]),
                (7, &[]),
            ],
        );
        let shape = calculate(&g).expect("shape");
        let mut order = Vec::new();
        rendered(&shape, &mut order);
        let unique = order.iter().copied().collect::<BTreeSet<_>>();
        assert_eq!(order.len(), unique.len(), "rendered twice: {order:?}");
        assert_eq!(unique, g.reachable());
    }

    #[test]
    fn unreachable_blocks_are_not_rendered() {
        let g = graph(1, &[(1, &[2]), (2, &[]), (9, &[2])]);
        let shape = calculate(&g).expect("shape");
        let mut order = Vec::new();
        rendered(&shape, &mut order);
        assert_eq!(order, vec![1, 2]);
    }

    #[test]
    fn reducible_graphs_are_recognized() {
        let reducible = graph(1, &[(1, &[2]), (2, &[3, 4]), (3, &[2]), (4, &[])]);
        assert!(reducible.is_reducible());
        let irreducible = graph(1, &[(1, &[2, 3]), (2, &[3]), (3, &[2])]);
        assert!(!irreducible.is_reducible());
    }

    fn dense_dominators(graph: &Graph) -> BTreeMap<Label, BTreeSet<Label>> {
        let reachable = graph.reachable();
        let mut dominators: BTreeMap<Label, BTreeSet<Label>> = reachable
            .iter()
            .map(|label| (*label, reachable.clone()))
            .collect();
        if let Some(entry) = dominators.get_mut(&graph.entry) {
            *entry = BTreeSet::from([graph.entry]);
        }
        let mut changed = true;
        while changed {
            changed = false;
            for label in &reachable {
                if *label == graph.entry {
                    continue;
                }
                let mut incoming = graph
                    .pred(*label)
                    .iter()
                    .filter(|predecessor| reachable.contains(predecessor))
                    .filter_map(|predecessor| dominators.get(predecessor));
                let Some(first) = incoming.next() else {
                    continue;
                };
                let mut next = first.clone();
                for other in incoming {
                    next.retain(|candidate| other.contains(candidate));
                }
                next.insert(*label);
                if dominators.get(label) != Some(&next) {
                    dominators.insert(*label, next);
                    changed = true;
                }
            }
        }
        dominators
    }

    fn assert_matches_dense(graph: &Graph, what: &str) {
        let dense = dense_dominators(graph);
        let tree = graph.dominators();
        let reachable = graph.reachable();
        for node in &reachable {
            for ancestor in &reachable {
                assert_eq!(
                    tree.dominates(*ancestor, *node),
                    dense[node].contains(ancestor),
                    "{what}: disagreement on whether %{ancestor} dominates %{node}"
                );
            }
        }
    }

    #[test]
    fn the_dominator_tree_agrees_with_the_dominator_sets() {
        for (what, graph) in [
            ("straight line", graph(1, &[(1, &[2]), (2, &[3]), (3, &[])])),
            (
                "diamond",
                graph(1, &[(1, &[2, 3]), (2, &[4]), (3, &[4]), (4, &[])]),
            ),
            (
                "loop with an exit",
                graph(1, &[(1, &[2]), (2, &[3, 4]), (3, &[2]), (4, &[])]),
            ),
            (
                "nested loops",
                graph(
                    1,
                    &[(1, &[2]), (2, &[3]), (3, &[3, 4]), (4, &[2, 5]), (5, &[])],
                ),
            ),
            ("self loop", graph(1, &[(1, &[1, 2]), (2, &[])])),
            (
                "irreducible pair",
                graph(1, &[(1, &[2, 3]), (2, &[3]), (3, &[2])]),
            ),
            (
                "unreachable block",
                graph(1, &[(1, &[2]), (2, &[]), (9, &[2])]),
            ),
            (
                "two paths rejoining under a loop",
                graph(
                    1,
                    &[
                        (1, &[2, 3]),
                        (2, &[4]),
                        (3, &[4]),
                        (4, &[5, 6]),
                        (5, &[4]),
                        (6, &[]),
                    ],
                ),
            ),
        ] {
            assert_matches_dense(&graph, what);
        }
    }

    #[test]
    fn the_dominator_structure_stays_linear_in_the_block_count() {
        let mut edges: Vec<(Label, Vec<Label>)> = Vec::new();
        let diamonds = 2_000;
        for index in 0..diamonds {
            let head = index * 4 + 1;
            edges.push((head, vec![head + 1, head + 2]));
            edges.push((head + 1, vec![head + 3]));
            edges.push((head + 2, vec![head + 3]));
            edges.push((head + 3, vec![head + 4]));
        }
        let last = diamonds * 4 + 1;
        edges.push((last, vec![]));
        let graph = Graph::new(
            1,
            edges.into_iter().collect::<BTreeMap<Label, Vec<Label>>>(),
        );
        let blocks = graph.reachable().len();
        assert_eq!(blocks, (diamonds * 4 + 1) as usize);
        assert!(graph.is_reducible());
        assert_eq!(graph.dominators().reached(), blocks);
    }
}
