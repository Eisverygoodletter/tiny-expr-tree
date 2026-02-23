use mask_tracked_array::{Mask, MaskTrackedArray};
use tiny_expr_tree::{
    BranchNode, ComputableBranch, ComputableLeaf, LeafNode, TinyExprTree,
    alloc_gen::ConstructableTreeBranch, make_tree_aliases,
};
#[derive(Clone)]
enum BooleanLeaf {
    True,
    False,
    InsertedValue,
}
impl ComputableLeaf for BooleanLeaf {
    type LeafContext = bool;
    type LeafOutput = bool;
    fn compute(&self, context: &Self::LeafContext) -> Self::LeafOutput {
        match self {
            Self::False => false,
            Self::True => true,
            Self::InsertedValue => *context,
        }
    }
}
#[derive(Clone)]
pub enum BooleanComparator {
    And,
    Or,
}
impl<BA, LA, BM, LM> ComputableBranch<BooleanLeaf, BA, LA, BM, LM> for BooleanComparator
where
    BA: MaskTrackedArray<BranchNode<Self, BM, LM>, MaskType = BM>,
    LA: MaskTrackedArray<LeafNode<BooleanLeaf>, MaskType = LM>,
    BM: Mask,
    LM: Mask,
{
    type BranchContext = bool;
    type BranchOutput = bool;
    fn compute<'a>(
        &self,
        context: &Self::BranchContext,
        controls: tiny_expr_tree::BranchControls<'a, Self, BooleanLeaf, BA, LA, BM, LM>,
    ) -> Self::BranchOutput {
        let out = match self {
            Self::And => {
                println!("AND COMPUTE");
                controls
                    .compute_all_branches(context)
                    .chain(controls.compute_all_leaves(context))
                    .inspect(|v| println!("Item was {}", v))
                    .all(std::convert::identity)
            }
            Self::Or => {
                println!("OR COMPUTE");
                controls
                    .compute_all_branches(context)
                    .chain(controls.compute_all_leaves(context))
                    .inspect(|v| println!("Item was {}", v))
                    .any(std::convert::identity)
            }
        };
        println!("OUTPUTTING {:?}", out);
        out
    }
}
make_tree_aliases!(MiniTree, BooleanComparator, BooleanLeaf, u8, u16);

#[cfg(feature = "alloc-gen")]
#[test]
fn basic() {
    let mut construction = ConstructableTreeBranch::new(BooleanComparator::Or);
    construction.add_leaf(BooleanLeaf::False);
    let mut sub_tree = ConstructableTreeBranch::new(BooleanComparator::And);
    sub_tree.add_leaf(BooleanLeaf::True);
    sub_tree.add_leaf(BooleanLeaf::InsertedValue);
    construction.add_branch(sub_tree.clone());
    let tree: MiniTree = construction.to_tree().unwrap();
    // assert!(tree.compute(&true));
    let sub_tree_compute: MiniTree = sub_tree.to_tree().unwrap();
    // assert!(!sub_tree_compute.compute(&false));
    println!("Subtree success");
    assert!(!tree.compute(&false));
}
