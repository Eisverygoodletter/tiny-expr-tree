//! [`ConstructableTreeBranch`] and [`ConstructableTreeLeaf`] used to construct
//! a [`Tree`]. This module requires the `alloc-gen` feature flag and will
//! require a global allocator.
//!
//! For constrained environments, the tree can be constructed
//! on a host computer with `alloc` available, then the `no-alloc` [`Tree`] is
//! sent to the microcontroller. Direct construction of a [`Tree`] is not
//! encouraged because removal of elements can be quite unperformant.
extern crate alloc;
use core::marker::PhantomData;

use alloc::boxed::Box;
use alloc::vec::Vec;
use mask_tracked_array::{Mask, MaskTrackedArray};

use crate::{BranchNode, ChildrenMask, LeafNode, TinyExprTree};
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstructableTreeBranch<B, L> {
    pub sub_branches: Vec<Box<ConstructableTreeBranch<B, L>>>,
    pub value: B,
    pub leaves: Vec<ConstructableTreeLeaf<L>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstructableTreeLeaf<L> {
    pub value: L,
}

struct AccumulatingVisitor<B, L, BA, LA, BM, LM>
where
    BA: MaskTrackedArray<BranchNode<B, BM, LM>, MaskType = BM>,
    LA: MaskTrackedArray<LeafNode<L>, MaskType = LM>,
{
    branches: BA,
    leaves: LA,
    _phantom: PhantomData<(B, L, BM, LM)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionError {
    InsufficientBranchCapacity,
    InsufficientLeafCapacity,
}

impl<B, L> ConstructableTreeBranch<B, L> {
    pub fn branch_count(&self) -> usize {
        self.sub_branches
            .iter()
            .map(|branch| branch.branch_count())
            .sum::<usize>()
            + 1
    }
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
            + self
                .sub_branches
                .iter()
                .map(|branch| branch.leaf_count())
                .sum::<usize>()
    }
    fn visit<BA, LA, BM, LM>(
        self,
        visitor: &mut AccumulatingVisitor<B, L, BA, LA, BM, LM>,
    ) -> Result<BM, ConstructionError>
    where
        BA: MaskTrackedArray<BranchNode<B, BM, LM>, MaskType = BM>,
        LA: MaskTrackedArray<LeafNode<L>, MaskType = LM>,
        BM: Mask,
        LM: Mask,
    {
        let branch_mask: Result<BM, ConstructionError> = self
            .sub_branches
            .into_iter()
            .map(|branch| branch.visit(visitor))
            .try_fold(<BM as Mask>::NONE_SELECTED, |acc, value| {
                Ok(acc | value.map_err(|_| ConstructionError::InsufficientBranchCapacity)?)
            });
        let leaf_mask: Result<LM, ConstructionError> = self
            .leaves
            .into_iter()
            .map(|leaf| visitor.leaves.push(LeafNode { leaf: leaf.value }))
            .try_fold(<LM as Mask>::NONE_SELECTED, |acc, value| {
                Ok(acc
                    | (<LM as Mask>::ONE_SELECTED
                        << value.map_err(|_| ConstructionError::InsufficientLeafCapacity)?))
            });
        let branch_mask = branch_mask?;
        let leaf_mask = leaf_mask?;
        let branch_node = BranchNode {
            branch: self.value,
            mask: ChildrenMask {
                branch_mask,
                leaf_mask,
            },
        };
        let this_index = visitor.branches.push(branch_node);
        this_index
            .map(|index| BM::ONE_SELECTED << index)
            .map_err(|_| ConstructionError::InsufficientBranchCapacity)
    }
    pub fn to_tree<BA, LA, BM, LM>(
        self,
    ) -> Result<TinyExprTree<B, L, BA, LA, BM, LM>, ConstructionError>
    where
        BA: MaskTrackedArray<BranchNode<B, BM, LM>, MaskType = BM>,
        LA: MaskTrackedArray<LeafNode<L>, MaskType = LM>,
        BM: Mask,
        LM: Mask,
    {
        let mut visitor = AccumulatingVisitor {
            _phantom: PhantomData,
            branches: BA::new(),
            leaves: LA::new(),
        };
        let branch_mask: Result<BM, ConstructionError> = self
            .sub_branches
            .into_iter()
            .map(|branch| branch.visit(&mut visitor))
            .try_fold(BM::NONE_SELECTED, |acc, value| {
                Ok(acc | value.map_err(|_| ConstructionError::InsufficientBranchCapacity)?)
            });
        let leaf_mask: Result<LM, ConstructionError> = self
            .leaves
            .into_iter()
            .map(|leaf| visitor.leaves.push(LeafNode { leaf: leaf.value }))
            .try_fold(LM::NONE_SELECTED, |acc, value| {
                Ok(acc
                    | (LM::ONE_SELECTED
                        << value.map_err(|_| ConstructionError::InsufficientLeafCapacity)?))
            });
        let branch_mask = branch_mask?;
        let leaf_mask = leaf_mask?;
        let branch_node = BranchNode {
            branch: self.value,
            mask: ChildrenMask {
                branch_mask,
                leaf_mask,
            },
        };
        Ok(TinyExprTree {
            inner: crate::TreeInner {
                branches: visitor.branches,
                leaves: visitor.leaves,
                _phantom: PhantomData,
            },
            root: branch_node,
        })
    }
    pub fn new(root: B) -> Self {
        Self {
            sub_branches: Vec::new(),
            value: root,
            leaves: Vec::new(),
        }
    }
    pub fn add_branch(&mut self, branch: Self) {
        self.sub_branches.push(Box::new(branch));
    }
    pub fn add_leaf(&mut self, leaf: L) {
        self.leaves.push(ConstructableTreeLeaf { value: leaf });
    }
}
