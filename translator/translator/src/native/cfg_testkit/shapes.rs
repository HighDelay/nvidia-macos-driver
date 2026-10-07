use super::build::CfgBuilder;
use crate::spirv_module::Module;
use spirv::Word;

const ACCUMULATOR_BOUND: u32 = 4096;

#[derive(Clone, Debug)]
enum Term {
    Branch(usize),
    Conditional(Cond, usize, usize),
    Multiway {
        mask: u32,
        arms: Vec<usize>,
        default: usize,
    },
    Return,
}

#[derive(Clone, Copy, Debug)]
enum Cond {
    MaskIsZero(u32),
    LessThan(u32),
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u32) -> u32 {
        (self.next() % u64::from(bound)) as u32
    }

    fn chance(&mut self, numerator: u32) -> bool {
        self.below(16) < numerator
    }
}

struct Generator {
    rng: Rng,
    terminators: Vec<Option<Term>>,
    settled: Vec<usize>,
    loop_headers: Vec<usize>,
    region_exits: Vec<usize>,
    escapes: u32,
    escape_budget: u32,
    cross_budget: u32,
}

impl Generator {
    fn allocate(&mut self) -> usize {
        self.terminators.push(None);
        self.terminators.len() - 1
    }

    fn set(&mut self, block: usize, term: Term) {
        assert!(
            self.terminators[block].is_none(),
            "block {block} terminated twice"
        );
        self.terminators[block] = Some(term);
        self.settled.push(block);
    }

    fn cross_edge(&mut self, from: usize) -> Option<(Cond, usize)> {
        if self.cross_budget == 0 {
            return None;
        }
        let candidates = self
            .settled
            .iter()
            .copied()
            .filter(|block| {
                *block != 0
                    && *block != from
                    && !self.loop_headers.contains(block)
                    && !self.region_exits.contains(block)
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        self.cross_budget -= 1;
        let pick = self.rng.below(candidates.len() as u32) as usize;
        let bound = ACCUMULATOR_BOUND - self.rng.below(64);
        Some((Cond::LessThan(bound), candidates[pick]))
    }

    fn escape_target(&mut self) -> Option<usize> {
        if self.escapes >= self.escape_budget {
            return None;
        }
        let candidates = self
            .region_exits
            .iter()
            .chain(&self.loop_headers)
            .copied()
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        self.escapes += 1;
        let pick = self.rng.below(candidates.len() as u32) as usize;
        Some(candidates[pick])
    }

    fn flow(&mut self, block: usize, next: usize) {
        let crossing = self.rng.chance(4);
        if crossing {
            if let Some((cond, target)) = self.cross_edge(block) {
                self.set(block, Term::Conditional(cond, target, next));
                return;
            }
        }
        if self.rng.chance(6) {
            if let Some(target) = self.escape_target() {
                let mask = 1 << self.rng.below(5);
                self.set(
                    block,
                    Term::Conditional(Cond::MaskIsZero(mask), target, next),
                );
                return;
            }
        }
        self.set(block, Term::Branch(next));
    }

    fn region(&mut self, entry: usize, exit: usize, depth: u32) {
        if depth == 0 {
            self.flow(entry, exit);
            return;
        }
        match self.rng.below(4) {
            0 => self.sequence(entry, exit, depth),
            1 => self.conditional(entry, exit, depth),
            2 => self.multiway(entry, exit, depth),
            _ => self.loop_region(entry, exit, depth),
        }
    }

    fn multiway(&mut self, entry: usize, exit: usize, depth: u32) {
        let arms = (0..3).map(|_| self.allocate()).collect::<Vec<_>>();
        let default = self.allocate();
        self.set(
            entry,
            Term::Multiway {
                mask: 3,
                arms: arms.clone(),
                default,
            },
        );
        self.region_exits.push(exit);
        for arm in arms.iter().chain(std::iter::once(&default)) {
            self.region(*arm, exit, depth - 1);
        }
        self.region_exits.pop();
    }

    fn sequence(&mut self, entry: usize, exit: usize, depth: u32) {
        let length = 1 + self.rng.below(3) as usize;
        let mut current = entry;
        for step in 0..length {
            let next = if step + 1 == length {
                exit
            } else {
                self.allocate()
            };
            self.region_exits.push(next);
            self.region(current, next, depth - 1);
            self.region_exits.pop();
            current = next;
        }
    }

    fn conditional(&mut self, entry: usize, exit: usize, depth: u32) {
        let on_true = self.allocate();
        let on_false = self.allocate();
        let mask = 1 << self.rng.below(5);
        self.set(
            entry,
            Term::Conditional(Cond::MaskIsZero(mask), on_true, on_false),
        );
        self.region_exits.push(exit);
        self.region(on_true, exit, depth - 1);
        self.region(on_false, exit, depth - 1);
        self.region_exits.pop();
    }

    fn loop_region(&mut self, header: usize, exit: usize, depth: u32) {
        let body = self.allocate();
        let latch = self.allocate();
        let bound = ACCUMULATOR_BOUND - self.rng.below(64);
        self.set(header, Term::Conditional(Cond::LessThan(bound), body, exit));
        self.loop_headers.push(header);
        self.region_exits.push(latch);
        self.region(body, latch, depth - 1);
        self.region_exits.pop();
        self.loop_headers.pop();
        self.set(latch, Term::Branch(header));
    }
}

pub(in crate::native) struct Shape {
    seed: u64,
    terminators: Vec<Term>,
}

impl Shape {
    pub(in crate::native) fn blocks(&self) -> usize {
        self.terminators.len()
    }

    pub(in crate::native) fn swapping_edges(&self) -> usize {
        predecessors(&self.terminators)
            .iter()
            .enumerate()
            .map(|(block, sources)| sources.iter().filter(|source| **source > block).count())
            .sum()
    }

    pub(in crate::native) fn branching(&self) -> usize {
        self.terminators
            .iter()
            .filter(|term| matches!(term, Term::Conditional(..) | Term::Multiway { .. }))
            .count()
    }
}

pub(in crate::native) fn shape(seed: u64, depth: u32) -> Shape {
    grow(seed, depth, 0)
}

pub(in crate::native) fn irreducible_shape(seed: u64, depth: u32, crossings: u32) -> Shape {
    grow(seed, depth, crossings)
}

fn grow(seed: u64, depth: u32, crossings: u32) -> Shape {
    let mut generator = Generator {
        rng: Rng(seed),
        terminators: Vec::new(),
        settled: Vec::new(),
        loop_headers: Vec::new(),
        region_exits: Vec::new(),
        escapes: 0,
        escape_budget: 24,
        cross_budget: crossings,
    };
    let entry = generator.allocate();
    let exit = generator.allocate();
    let start = generator.allocate();
    generator.set(entry, Term::Branch(start));
    generator.region(start, exit, depth);
    generator.set(exit, Term::Return);
    let terminators = generator
        .terminators
        .into_iter()
        .map(|term| term.expect("every allocated block is terminated"))
        .collect();
    Shape { seed, terminators }
}

pub(in crate::native) fn author(shape: &Shape) -> Module {
    author_with(shape, Seed::Parameter).0
}

pub(in crate::native) fn author_constant_seeded(shape: &Shape, value: u32) -> (Module, Word) {
    let (module, result) = author_with(shape, Seed::PrivateConstant(value));
    (
        module,
        result.expect("a constant-seeded function stores its answer"),
    )
}

enum Seed {
    Parameter,
    PrivateConstant(u32),
}

fn author_with(shape: &Shape, seed: Seed) -> (Module, Option<Word>) {
    let mut builder = match seed {
        Seed::Parameter => CfgBuilder::new(1),
        Seed::PrivateConstant(_) => CfgBuilder::new_entry_point(),
    };
    let (global, result_global) = match seed {
        Seed::Parameter => (None, None),
        Seed::PrivateConstant(value) => (
            Some(builder.private_global(value)),
            Some(builder.private_global(0)),
        ),
    };
    let name = |block: usize| format!("b{block}");

    let predecessors = predecessors(&shape.terminators);
    let blocks = shape.terminators.len();
    let accumulator = (0..blocks)
        .map(|_| builder.reserve_value())
        .collect::<Vec<_>>();
    let mixed = (0..blocks)
        .map(|_| builder.reserve_value())
        .collect::<Vec<_>>();
    let carried = (0..blocks)
        .map(|_| builder.reserve_value())
        .collect::<Vec<_>>();

    for (block, term) in shape.terminators.iter().enumerate() {
        builder.block(&name(block));
        let (accumulator_in, mixed_in, carried_in) = if block == 0 {
            assert!(
                predecessors[0].is_empty(),
                "the entry block has predecessors, so its values would ignore them"
            );
            let seed = match global {
                Some(global) => builder.load(global),
                None => builder.parameter(0),
            };
            let five = builder.constant(5);
            let nine = builder.constant(9);
            let seeded_mix = builder.bitwise_xor(seed, five);
            let seeded_carry = builder.add(seed, nine);
            (seed, seeded_mix, seeded_carry)
        } else {
            let sources = &predecessors[block];
            assert!(
                !sources.is_empty(),
                "block {block} is unreachable, so the shape is not connected"
            );
            let swaps = |predecessor: &usize| *predecessor > block;
            let names = sources.iter().map(|p| name(*p)).collect::<Vec<_>>();
            let pick = |values: &[Word], swapped: &[Word]| {
                sources
                    .iter()
                    .enumerate()
                    .map(|(index, predecessor)| {
                        let value = if swaps(predecessor) {
                            swapped[*predecessor]
                        } else {
                            values[*predecessor]
                        };
                        (value, names[index].as_str())
                    })
                    .collect::<Vec<_>>()
            };
            let straight = sources
                .iter()
                .enumerate()
                .map(|(index, predecessor)| (accumulator[*predecessor], names[index].as_str()))
                .collect::<Vec<_>>();
            let crossed_mix = pick(&mixed, &carried);
            let crossed_carry = pick(&carried, &mixed);
            let one_of = |builder: &mut CfgBuilder, incoming: &[(Word, &str)]| {
                if incoming.len() == 1 {
                    incoming[0].0
                } else {
                    builder.phi(incoming)
                }
            };
            (
                one_of(&mut builder, &straight),
                one_of(&mut builder, &crossed_mix),
                one_of(&mut builder, &crossed_carry),
            )
        };

        let step = builder.constant(block as u32 + 1);
        let value = accumulator[block];
        builder.add_into(value, accumulator_in, step);
        builder.bitwise_xor_into(mixed[block], mixed_in, value);
        builder.add_into(carried[block], carried_in, value);

        match term {
            Term::Branch(target) => builder.branch(&name(*target)),
            Term::Conditional(cond, on_true, on_false) => {
                let condition = match cond {
                    Cond::MaskIsZero(mask) => {
                        let mask = builder.constant(*mask);
                        let masked = builder.bitwise_and(value, mask);
                        let zero = builder.constant(0);
                        builder.equal(masked, zero)
                    }
                    Cond::LessThan(bound) => {
                        let bound = builder.constant(*bound);
                        builder.less_than(value, bound)
                    }
                };
                builder.branch_conditional(condition, &name(*on_true), &name(*on_false));
            }
            Term::Multiway {
                mask,
                arms,
                default,
            } => {
                let mask = builder.constant(*mask);
                let selector = builder.bitwise_and(value, mask);
                let cases = arms
                    .iter()
                    .enumerate()
                    .map(|(literal, arm)| (literal as u32, name(*arm)))
                    .collect::<Vec<_>>();
                let cases = cases
                    .iter()
                    .map(|(literal, target)| (*literal, target.as_str()))
                    .collect::<Vec<_>>();
                builder.switch(selector, &name(*default), &cases);
            }
            Term::Return => {
                let summed = builder.add(value, mixed[block]);
                let result = builder.bitwise_xor(summed, carried[block]);
                match result_global {
                    Some(slot) => {
                        builder.store(slot, result);
                        builder.return_void();
                    }
                    None => builder.return_value(result),
                }
            }
        }
    }
    (builder.finish(), result_global)
}

fn predecessors(terminators: &[Term]) -> Vec<Vec<usize>> {
    let mut predecessors = vec![Vec::new(); terminators.len()];
    for (block, term) in terminators.iter().enumerate() {
        let mut edge = |target: usize| {
            if !predecessors[target].contains(&block) {
                predecessors[target].push(block);
            }
        };
        match term {
            Term::Branch(target) => edge(*target),
            Term::Conditional(_, on_true, on_false) => {
                edge(*on_true);
                edge(*on_false);
            }
            Term::Multiway { arms, default, .. } => {
                for arm in arms {
                    edge(*arm);
                }
                edge(*default);
            }
            Term::Return => {}
        }
    }
    predecessors
}

pub(in crate::native) fn describe(shape: &Shape) -> String {
    let header = format!(
        "seed {} ({} blocks, {} branching)",
        shape.seed,
        shape.blocks(),
        shape.branching()
    );
    let blocks = shape
        .terminators
        .iter()
        .enumerate()
        .map(|(block, term)| match term {
            Term::Branch(target) => format!("b{block} -> b{target}"),
            Term::Conditional(cond, on_true, on_false) => {
                format!("b{block} -> {cond:?} ? b{on_true} : b{on_false}")
            }
            Term::Multiway {
                mask,
                arms,
                default,
            } => {
                let arms = arms
                    .iter()
                    .enumerate()
                    .map(|(literal, arm)| format!("{literal} => b{arm}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("b{block} -> switch (acc & {mask}) {{ {arms}, _ => b{default} }}")
            }
            Term::Return => format!("b{block} return"),
        });
    std::iter::once(header)
        .chain(blocks)
        .collect::<Vec<_>>()
        .join("\n")
}
