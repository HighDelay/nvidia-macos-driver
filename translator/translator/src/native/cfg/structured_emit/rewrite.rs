use super::BodyBlock;

pub(in crate::native) fn atomic_rewrite<T>(
    blocks: &mut Vec<BodyBlock>,
    rewrite: impl FnOnce(&mut Vec<BodyBlock>) -> Option<T>,
) -> Option<T> {
    let mut staged = blocks.clone();
    let outcome = rewrite(&mut staged)?;
    *blocks = staged;
    Some(outcome)
}

pub(in crate::native) struct RewriteBlocks {
    blocks: Vec<BodyBlock>,
    revision: u64,
}

impl RewriteBlocks {
    pub(in crate::native) fn new(blocks: Vec<BodyBlock>) -> Self {
        Self {
            blocks,
            revision: 0,
        }
    }

    pub(in crate::native) fn get(&self) -> &[BodyBlock] {
        &self.blocks
    }

    pub(in crate::native) fn revision(&self) -> u64 {
        self.revision
    }

    pub(in crate::native) fn rewrite<T>(
        &mut self,
        rewrite: impl FnOnce(&mut Vec<BodyBlock>) -> Option<T>,
    ) -> Option<T> {
        #[cfg(debug_assertions)]
        let before = decline_witness(&self.blocks);
        match rewrite(&mut self.blocks) {
            Some(outcome) => {
                self.revision += 1;
                Some(outcome)
            }
            None => {
                #[cfg(debug_assertions)]
                assert!(
                    before == decline_witness(&self.blocks),
                    "a declined rewrite edited the block list; wrap it in atomic_rewrite"
                );
                None
            }
        }
    }

    pub(in crate::native) fn edit(&mut self) -> &mut Vec<BodyBlock> {
        self.revision += 1;
        &mut self.blocks
    }

    pub(in crate::native) fn into_inner(self) -> Vec<BodyBlock> {
        self.blocks
    }
}

#[cfg(debug_assertions)]
fn decline_witness(blocks: &[BodyBlock]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    blocks.len().hash(&mut hasher);
    for block in blocks {
        block.name.hash(&mut hasher);
        super::block_successors(block).hash(&mut hasher);
    }
    hasher.finish()
}

pub(in crate::native) struct AnalysisCache<T> {
    entry: Option<(u64, T)>,
}

impl<T> Default for AnalysisCache<T> {
    fn default() -> Self {
        Self { entry: None }
    }
}

impl<T> AnalysisCache<T> {
    pub(in crate::native) fn get<'cache>(
        &'cache mut self,
        blocks: &RewriteBlocks,
        derive: impl FnOnce(&[BodyBlock]) -> T,
    ) -> &'cache T {
        let revision = blocks.revision();
        if !matches!(&self.entry, Some((cached, _)) if *cached == revision) {
            self.entry = Some((revision, derive(blocks.get())));
        }
        &self.entry.as_ref().expect("just derived").1
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::bb;
    use super::*;

    fn one_block() -> Vec<BodyBlock> {
        vec![bb("%entry", &["ret void"])]
    }

    #[test]
    fn only_a_committed_rewrite_advances_the_revision() {
        let mut blocks = RewriteBlocks::new(one_block());
        let start = blocks.revision();

        assert_eq!(blocks.rewrite(|_| None::<()>), None);
        assert_eq!(blocks.revision(), start, "a decline is not a change");

        assert_eq!(
            blocks.rewrite(|blocks| {
                blocks.push(bb("%tail", &["ret void"]));
                Some(())
            }),
            Some(())
        );
        assert_ne!(blocks.revision(), start, "a commit is a change");
        assert_eq!(blocks.get().len(), 2);
    }

    #[test]
    fn unreported_edit_access_advances_the_revision() {
        let mut blocks = RewriteBlocks::new(one_block());
        let start = blocks.revision();
        let _ = blocks.edit();
        assert_ne!(blocks.revision(), start);
    }

    #[test]
    fn an_analysis_is_derived_once_per_revision() {
        let mut blocks = RewriteBlocks::new(one_block());
        let mut cache = AnalysisCache::default();
        let mut derivations = 0usize;

        for _ in 0..3 {
            assert_eq!(
                *cache.get(&blocks, |b| {
                    derivations += 1;
                    b.len()
                }),
                1
            );
        }
        assert_eq!(derivations, 1, "an unchanged graph is analyzed once");

        blocks.edit().push(bb("%tail", &["ret void"]));
        assert_eq!(
            *cache.get(&blocks, |b| {
                derivations += 1;
                b.len()
            }),
            2
        );
        assert_eq!(derivations, 2, "a changed graph is analyzed again");
    }

    #[test]
    #[should_panic(expected = "a declined rewrite edited the block list")]
    #[cfg(debug_assertions)]
    fn a_declined_rewrite_that_edited_the_list_is_caught() {
        let mut blocks = RewriteBlocks::new(one_block());
        blocks.rewrite(|blocks| {
            blocks.push(bb("%tail", &["ret void"]));
            None::<()>
        });
    }
}
