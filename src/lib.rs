#![no_std]
use core::marker::PhantomData;

use mask_tracked_array::MaskTrackedArray;
#[cfg(feature = "alloc-gen")]
pub mod alloc_gen;
pub trait ComputableBranch<L, BM, LM>
where
    Self: Sized,
    BM: MaskTrackedArray<BranchNode<Self, L, BM, LM>>,
    LM: MaskTrackedArray<LeafNode<L>>,
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
        controls: BranchControls<'a, Self, L, BM, LM>,
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

pub struct ChildrenMask<
    B,
    L,
    BM: MaskTrackedArray<BranchNode<B, L, BM, LM>>,
    LM: MaskTrackedArray<LeafNode<L>>,
> {
    pub branch_mask: BM::MaskType,
    pub leaf_mask: LM::MaskType,
}
impl<B, L, BM, LM> Clone for ChildrenMask<B, L, BM, LM>
where
    BM: MaskTrackedArray<BranchNode<B, L, BM, LM>>,
    LM: MaskTrackedArray<LeafNode<L>>,
{
    fn clone(&self) -> Self {
        Self {
            branch_mask: self.branch_mask.clone(),
            leaf_mask: self.leaf_mask.clone(),
        }
    }
}
impl<B, L, BM, LM> Copy for ChildrenMask<B, L, BM, LM>
where
    BM: MaskTrackedArray<BranchNode<B, L, BM, LM>>,
    LM: MaskTrackedArray<LeafNode<L>>,
{
}

pub struct BranchNode<
    B,
    L,
    BM: MaskTrackedArray<BranchNode<B, L, BM, LM>>,
    LM: MaskTrackedArray<LeafNode<L>>,
> {
    branch: B,
    mask: ChildrenMask<B, L, BM, LM>,
}
pub struct LeafNode<L> {
    leaf: L,
}

pub struct TreeInner<B, L, BM, LM>
where
    BM: MaskTrackedArray<BranchNode<B, L, BM, LM>>,
    LM: MaskTrackedArray<LeafNode<L>>,
{
    branches: BM,
    leaves: LM,
    _phantom: PhantomData<(B, L)>,
}
pub struct Tree<B, L, BM, LM>
where
    BM: MaskTrackedArray<BranchNode<B, L, BM, LM>>,
    LM: MaskTrackedArray<LeafNode<L>>,
{
    root: BranchNode<B, L, BM, LM>,
    inner: TreeInner<B, L, BM, LM>,
}
pub struct BranchControls<'a, B, L, BM, LM>
where
    BM: MaskTrackedArray<BranchNode<B, L, BM, LM>>,
    LM: MaskTrackedArray<LeafNode<L>>,
{
    inner_reference: &'a TreeInner<B, L, BM, LM>,
    mask: ChildrenMask<B, L, BM, LM>,
}

impl<'a, B, L, BM, LM> BranchControls<'a, B, L, BM, LM>
where
    B: ComputableBranch<L, BM, LM>,
    L: ComputableLeaf,
    BM: MaskTrackedArray<BranchNode<B, L, BM, LM>>,
    LM: MaskTrackedArray<LeafNode<L>>,
{
    pub fn compute_branches(
        &'a mut self,
        context: &B::BranchContext,
        mask: BM::MaskType,
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
    pub fn compute_leaves(
        &mut self,
        context: &L::LeafContext,
        mask: LM::MaskType,
    ) -> impl Iterator<Item = L::LeafOutput> {
        self.inner_reference
            .leaves
            .iter_filled_indices_mask(mask)
            .map(|index| {
                let leaf = unsafe { self.inner_reference.leaves.get_unchecked_mut(index) };
                leaf.leaf.compute(context)
            })
    }
}
impl<B, L, BM, LM> Tree<B, L, BM, LM>
where
    B: ComputableBranch<L, BM, LM>,
    L: ComputableLeaf,
    BM: MaskTrackedArray<BranchNode<B, L, BM, LM>>,
    LM: MaskTrackedArray<LeafNode<L>>,
{
    pub fn compute(&mut self, context: &B::BranchContext) -> B::BranchOutput {
        let base_access = BranchControls {
            inner_reference: &self.inner,
            mask: self.root.mask,
        };
        self.root.branch.compute(context, base_access)
    }
}
