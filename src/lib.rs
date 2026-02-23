#![no_std]
#![doc=include_str!("../README.md")]
use core::marker::PhantomData;

use mask_tracked_array::{Mask, MaskTrackedArray};

#[cfg(feature = "alloc-gen")]
pub mod alloc_gen;
/// Should be implemented on branch node structs. Sub-branch/leaf access is
/// provided by [`BranchControls`] so you should not hold references to
/// branches and other items.
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
/// Should be implemented on leaf nodes structs.
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
struct TreeInner<B, L, BA, LA, BM, LM>
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
/// A tiny tree suitable for microcontroller use. This struct is not directly
/// constructable and you should use [`alloc_gen::ConstructableTreeBranch`]s
/// instead on the host computer.
pub struct TinyExprTree<B, L, BA, LA, BM, LM>
where
    BA: MaskTrackedArray<BranchNode<B, BM, LM>, MaskType = BM>,
    LA: MaskTrackedArray<LeafNode<L>, MaskType = LM>,
{
    root: BranchNode<B, BM, LM>,
    inner: TreeInner<B, L, BA, LA, BM, LM>,
}
#[derive(Debug)]
/// Provides compute actions for [`ComputableBranch`]es and access to
/// sub-branches and leaves.
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
    /// Mask representing sub-branches
    #[inline]
    pub fn branch_mask(&self) -> BM {
        self.mask.branch_mask
    }
    /// Mask representing leaves
    #[inline]
    pub fn leaf_mask(&self) -> LM {
        self.mask.leaf_mask
    }
    /// Check if there are any sub-branches
    #[inline]
    pub fn has_branches(&self) -> bool {
        self.mask.branch_mask != BM::NONE_SELECTED
    }
    /// Check if there are any leaves
    #[inline]
    pub fn has_leaves(&self) -> bool {
        self.mask.leaf_mask != LM::NONE_SELECTED
    }
    /// Compute the value of all sub-branches specified in the mask.
    #[inline]
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
    /// Compute the value of all sub-branches
    #[inline]
    pub fn compute_all_branches(
        &self,
        context: &B::BranchContext,
    ) -> impl Iterator<Item = <B as ComputableBranch<L, BA, LA, BM, LM>>::BranchOutput> {
        self.compute_branches(context, <BA::MaskType as Mask>::ALL_SELECTED)
    }
    /// Compute the value of sub-leaves specified in the mask
    #[inline]
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
    /// Compute the values of all leaves
    #[inline]
    pub fn compute_all_leaves(
        &self,
        context: &L::LeafContext,
    ) -> impl Iterator<Item = <L as ComputableLeaf>::LeafOutput> {
        self.compute_leaves(context, <LM as Mask>::ALL_SELECTED)
    }
}

impl<'a, B, L, BA, LA, BM, LM> BranchControls<'a, B, L, BA, LA, BM, LM>
where
    B: ComputableBranch<L, BA, LA, BM, LM>,
    L: ComputableLeaf<LeafContext = B::BranchContext, LeafOutput = B::BranchOutput>,
    BM: Mask,
    BA: MaskTrackedArray<BranchNode<B, BM, LM>, MaskType = BM>,
    LA: MaskTrackedArray<LeafNode<L>, MaskType = LM>,
    LM: Mask,
{
    /// Compute the values of sub-branches and leaves specified in the masks
    #[inline]
    pub fn compute_both(
        &self,
        context: &B::BranchContext,
        branch_mask: BM,
        leaf_mask: LM,
    ) -> impl Iterator<Item = B::BranchOutput> {
        self.compute_branches(context, branch_mask)
            .chain(self.compute_leaves(context, leaf_mask))
    }
    /// Compute the values of all sub-branches and leaves
    #[inline]
    pub fn compute_all_both(
        &self,
        context: &B::BranchContext,
    ) -> impl Iterator<Item = B::BranchOutput> {
        self.compute_both(context, BM::ALL_SELECTED, LM::ALL_SELECTED)
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
    /// Compute a value using the given context starting at the root node. The
    /// exact operations done is dependent on the [`ComputableBranch`] and
    /// [`ComputableLeaf`] implementations you supply.
    pub fn compute(&self, context: &B::BranchContext) -> B::BranchOutput {
        let base_access = BranchControls {
            inner_reference: &self.inner,
            mask: self.root.mask,
        };
        self.root.branch.compute(context, base_access)
    }
}

/// Makes type aliases for [`TinyExprTree`] to make naming them easier especially
/// with the generics. This macro expects the following as its argument:
/// 1. Identifier for the alias.
/// 2. The type you implemented [`ComputableBranch`] on.
/// 3. The type you implemented [`ComputableLeaf`] on.
/// 4. The mask type for branch nodes.
/// 5. The mask type for leaf nodes.
///
/// The capacity of the tree for branch and leaf nodes is equal to the number
/// of bits in the branch and leaf node masks.
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
        make_tree_aliases!(@LA_GENERATION LA, $leaf_node, $lm);
        type $tree_ident = TinyExprTree<$branch_node, $leaf_node, BA, LA, $bm, $lm>;
    };
}
