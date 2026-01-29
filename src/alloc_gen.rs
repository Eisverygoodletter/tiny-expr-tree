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
use mask_tracked_array::MaskTrackedArray;
use num_traits::{Bounded, ConstOne};

use crate::{BranchNode, ChildrenMask, LeafNode, Tree};
pub struct ConstructableTreeBranch<B, L> {
    pub sub_branches: Vec<Box<ConstructableTreeBranch<B, L>>>,
    pub value: B,
    pub leaves: Vec<ConstructableTreeLeaf<L>>,
}
pub struct ConstructableTreeLeaf<L> {
    pub value: L,
}

struct AccumulatingVisitor<B, L, BM, LM>
where
    BM: MaskTrackedArray<BranchNode<B, L, BM, LM>>,
    LM: MaskTrackedArray<LeafNode<L>>,
{
    branches: BM,
    leaves: LM,
    _phantom: PhantomData<(B, L)>,
}

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
    fn visit<BM, LM>(
        self,
        visitor: &mut AccumulatingVisitor<B, L, BM, LM>,
    ) -> Result<BM::MaskType, ConstructionError>
    where
        BM: MaskTrackedArray<BranchNode<B, L, BM, LM>>,
        LM: MaskTrackedArray<LeafNode<L>>,
    {
        let branch_mask: Result<
            <BM as MaskTrackedArray<BranchNode<B, L, BM, LM>>>::MaskType,
            ConstructionError,
        > = self
            .sub_branches
            .into_iter()
            .map(|branch| branch.visit(visitor))
            .try_fold(<BM::MaskType as Bounded>::min_value(), |acc, value| {
                Ok(acc | value.map_err(|_| ConstructionError::InsufficientBranchCapacity)?)
            });
        let leaf_mask: Result<<LM as MaskTrackedArray<LeafNode<L>>>::MaskType, ConstructionError> =
            self.leaves
                .into_iter()
                .map(|leaf| visitor.leaves.push(LeafNode { leaf: leaf.value }))
                .try_fold(<LM::MaskType as Bounded>::min_value(), |acc, value| {
                    Ok(acc
                        | (<LM::MaskType as ConstOne>::ONE
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
            .map(|index| <BM::MaskType as ConstOne>::ONE << index)
            .map_err(|_| ConstructionError::InsufficientBranchCapacity)
    }
    pub fn to_tree<BM, LM>(self) -> Result<Tree<B, L, BM, LM>, ConstructionError>
    where
        BM: MaskTrackedArray<BranchNode<B, L, BM, LM>>,
        LM: MaskTrackedArray<LeafNode<L>>,
    {
        let mut visitor = AccumulatingVisitor {
            _phantom: PhantomData,
            branches: BM::new(),
            leaves: LM::new(),
        };
        let branch_mask: Result<
            <BM as MaskTrackedArray<BranchNode<B, L, BM, LM>>>::MaskType,
            ConstructionError,
        > = self
            .sub_branches
            .into_iter()
            .map(|branch| branch.visit(&mut visitor))
            .try_fold(<BM::MaskType as Bounded>::min_value(), |acc, value| {
                Ok(acc | value.map_err(|_| ConstructionError::InsufficientBranchCapacity)?)
            });
        let leaf_mask: Result<<LM as MaskTrackedArray<LeafNode<L>>>::MaskType, ConstructionError> =
            self.leaves
                .into_iter()
                .map(|leaf| visitor.leaves.push(LeafNode { leaf: leaf.value }))
                .try_fold(<LM::MaskType as Bounded>::min_value(), |acc, value| {
                    Ok(acc
                        | (<LM::MaskType as ConstOne>::ONE
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
        Ok(Tree {
            inner: crate::TreeInner {
                branches: visitor.branches,
                leaves: visitor.leaves,
                _phantom: PhantomData,
            },
            root: branch_node,
        })
    }
    pub fn new(root: B) -> Self {
        Self { sub_branches: Vec::new(), value: root, leaves: Vec::new() }
    }
}
