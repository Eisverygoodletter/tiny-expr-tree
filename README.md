# `no-std` `no-alloc` trees for embedded applications
Microcontroller programs often have to process some data before outputting it via some interface. Writing a customisable configuration engine for changing outputs based on user needs is very time consuming. This library provides an evaluatable tree structure that can be loaded on microcontrollers.

Every branch and leaf in this a `TinyExprTree` can be evaluated to a value. Branches can also access values computed from their leaf nodes.


A `TinyExprTree<B, L, BM, LM>` contains generic branch node `B` and leaf node `L` which you can customize. If you implement `ComputableBranch<L, BM, LM> for B` and `ComputableLeaf for L`, the `compute(&mut self, context: &B::Context) -> B::BranchOutput` method becomes available for the whole `TinyExprTree`.

You cannot directly construct a `TinyExprTree`, but can turn a `alloc_gen::ConstructableTreeBranch` into one.

