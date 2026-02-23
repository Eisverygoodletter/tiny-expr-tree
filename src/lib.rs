#![no_std]
#![doc=include_str!("../README.md")]
use core::marker::PhantomData;

use mask_tracked_array::{Mask, MaskTrackedArray};

#[cfg(feature = "alloc-gen")]
pub mod alloc_gen;
pub trait ComputableBranch<L, BA, LA, BM, LM>
where
    Self: Sized,
    BA: MaskTrackedArray<BranchNode<Self, BM, LM>, MaskType = BM>,
    LA: MaskTrackedArray<LeafNode<L>, MaskType = LM>,
{
    /// The context required to compute a branch node.
    type BranchContext;
    /// Output from computing.
    type BranchOutput;
    /// Compute the value inside the branch node. [`BranchControls`] are available
    /// for accessing items in subnodes.
    fn compute<'a>(
        &self,
        context: &Self::BranchContext,
        controls: BranchControls<'a, Self, L, BA, LA, BM, LM>,
    ) -> Self::BranchOutput;
}
pub trait ComputableLeaf {
    /// Context required to compute a leaf node.
    type LeafContext;
    /// Output from computing.
    type LeafOutput;
    /// Compute the value inside the leaf node using the context.
    fn compute(&self, context: &Self::LeafContext) -> Self::LeafOutput;
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy)]
pub struct ChildrenMask<BM, LM> {
    pub branch_mask: BM,
    pub leaf_mask: LM,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct BranchNode<B, BM, LM> {
    branch: B,
    mask: ChildrenMask<BM, LM>,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct LeafNode<L> {
    leaf: L,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct TreeInner<B, L, BA, LA, BM, LM>
where
    BA: MaskTrackedArray<BranchNode<B, BM, LM>, MaskType = BM>,
    LA: MaskTrackedArray<LeafNode<L>, MaskType = LM>,
{
    branches: BA,
    leaves: LA,
    _phantom: PhantomData<(B, L, BM)>,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct TinyExprTree<B, L, BA, LA, BM, LM>
where
    BA: MaskTrackedArray<BranchNode<B, BM, LM>, MaskType = BM>,
    LA: MaskTrackedArray<LeafNode<L>, MaskType = LM>,
{
    root: BranchNode<B, BM, LM>,
    inner: TreeInner<B, L, BA, LA, BM, LM>,
}
#[derive(Debug)]
pub struct BranchControls<'a, B, L, BA, LA, BM, LM>
where
    BA: MaskTrackedArray<BranchNode<B, BM, LM>, MaskType = BM>,
    LA: MaskTrackedArray<LeafNode<L>, MaskType = LM>,
{
    inner_reference: &'a TreeInner<B, L, BA, LA, BM, LM>,
    mask: ChildrenMask<BA::MaskType, LA::MaskType>,
}

impl<'a, B, L, BA, LA, BM, LM> BranchControls<'a, B, L, BA, LA, BM, LM>
where
    B: ComputableBranch<L, BA, LA, BM, LM>,
    L: ComputableLeaf,
    BM: Mask,
    BA: MaskTrackedArray<BranchNode<B, BM, LM>, MaskType = BM>,
    LA: MaskTrackedArray<LeafNode<L>, MaskType = LM>,
    LM: Mask,
{
    pub fn get_branch_mask(&self) -> BM {
        self.mask.branch_mask
    }
    pub fn get_leaf_mask(&self) -> LM {
        self.mask.leaf_mask
    }
    #[inline]
    pub fn has_branches(&self) -> bool {
        self.mask.branch_mask != BM::NONE_SELECTED
    }
    #[inline]
    pub fn has_leaves(&self) -> bool {
        self.mask.leaf_mask != LM::NONE_SELECTED
    }

    pub fn compute_branches(
        &self,
        context: &B::BranchContext,
        mask: BA::MaskType,
    ) -> impl Iterator<Item = B::BranchOutput> {
        let branch_control_mask = self.mask.branch_mask;
        let indices_iter = self
            .inner_reference
            .branches
            .iter_filled_indices_mask(mask & branch_control_mask);
        indices_iter.map(|index| {
            let branch = unsafe { self.inner_reference.branches.get_unchecked_mut(index) };
            let mask = branch.mask;
            let controls = BranchControls {
                inner_reference: self.inner_reference,
                mask,
            };
            branch.branch.compute(context, controls)
        })
    }
    pub fn compute_all_branches(
        &self,
        context: &B::BranchContext,
    ) -> impl Iterator<Item = <B as ComputableBranch<L, BA, LA, BM, LM>>::BranchOutput> {
        self.compute_branches(context, <BA::MaskType as Mask>::ALL_SELECTED)
    }
    pub fn compute_leaves(
        &self,
        context: &L::LeafContext,
        mask: LA::MaskType,
    ) -> impl Iterator<Item = L::LeafOutput> {
        self.inner_reference
            .leaves
            .iter_filled_indices_mask(mask & self.mask.leaf_mask)
            .map(|index| {
                let leaf = unsafe { self.inner_reference.leaves.get_unchecked_mut(index) };
                leaf.leaf.compute(context)
            })
    }
    pub fn compute_all_leaves(
        &self,
        context: &L::LeafContext,
    ) -> impl Iterator<Item = <L as ComputableLeaf>::LeafOutput> {
        self.compute_leaves(context, <LM as Mask>::ALL_SELECTED)
    }
}

impl<B, L, BA, LA, BM, LM> TinyExprTree<B, L, BA, LA, BM, LM>
where
    B: ComputableBranch<L, BA, LA, BM, LM>,
    L: ComputableLeaf,
    BA: MaskTrackedArray<BranchNode<B, BM, LM>, MaskType = BM>,
    LA: MaskTrackedArray<LeafNode<L>, MaskType = LM>,
    BM: Mask,
    LM: Mask,
{
    pub fn compute(&self, context: &B::BranchContext) -> B::BranchOutput {
        let base_access = BranchControls {
            inner_reference: &self.inner,
            mask: self.root.mask,
        };
        self.root.branch.compute(context, base_access)
    }
}

#[macro_export]
macro_rules! make_tree_aliases {
    (@BA_GENERATION $alias_name:ident, $branch_node:ty, $leaf_node:ty, $lm:ty, u8) => {
        type $alias_name = mask_tracked_array::MaskTrackedArrayU8<BranchNode<$branch_node, u8, $lm>>;
    };
    (@BA_GENERATION $alias_name:ident, $branch_node:ty, $leaf_node:ty, $lm:ty, u16) => {
        type $alias_name = mask_tracked_array::MaskTrackedArrayU16<BranchNode<$branch_node, u16, $lm>>;
    };
    (@BA_GENERATION $alias_name:ident, $branch_node:ty, $leaf_node:ty, $lm:ty, u32) => {
        type $alias_name = mask_tracked_array::MaskTrackedArrayU32<BranchNode<$branch_node, u32, $lm>>;
    };
    (@BA_GENERATION $alias_name:ident, $branch_node:ty, $leaf_node:ty, $lm:ty, u64) => {
        type $alias_name = mask_tracked_array::MaskTrackedArrayU64<BranchNode<$branch_node, u64, $lm>>;
    };
    (@BA_GENERATION $alias_name:ident, $branch_node:ty, $leaf_node:ty, $lm:ty, u128) => {
        type $alias_name = mask_tracked_array::MaskTrackedArrayU128<BranchNode<$branch_node, u128, $lm>>;
    };
    (@LA_GENERATION $alias_name:ident, $leaf_node:ty, u8) => {
        type $alias_name = mask_tracked_array::MaskTrackedArrayU8<LeafNode<$leaf_node>>;
    };
    (@LA_GENERATION $alias_name:ident, $leaf_node:ty, u16) => {
        type $alias_name = mask_tracked_array::MaskTrackedArrayU16<LeafNode<$leaf_node>>;
    };
    (@LA_GENERATION $alias_name:ident, $leaf_node:ty, u32) => {
        type $alias_name = mask_tracked_array::MaskTrackedArrayU32<LeafNode<$leaf_node>>;
    };
    (@LA_GENERATION $alias_name:ident, $leaf_node:ty, u64) => {
        type $alias_name = mask_tracked_array::MaskTrackedArrayU64<LeafNode<$leaf_node>>;
    };
    (@LA_GENERATION $alias_name:ident, $leaf_node:ty, u128) => {
        type $alias_name = mask_tracked_array::MaskTrackedArrayU128<LeafNode<$leaf_node>>;
    };
    ($tree_ident:ident, $branch_node:ty, $leaf_node:ty, $bm:tt, $lm:tt) => {
        make_tree_aliases!(@BA_GENERATION BA, $branch_node, $leaf_node, $lm, $bm);
        // type BA = MaskTrackedArrayU8<BranchNode<$branch_node, $bm, $lm>>;
        make_tree_aliases!(@LA_GENERATION LA, $leaf_node, $lm);
        // type LA = MaskTrackedArrayU8<LeafNode<$leaf_node>>;
        type $tree_ident = TinyExprTree<$branch_node, $leaf_node, BA, LA, $bm, $lm>;
    };
}
