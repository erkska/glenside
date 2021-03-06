use egg::{define_language, merge_if_different, EGraph, Id};
use itertools::{multizip, EitherOrBoth::*, Itertools};
use log::warn;
use ndarray::{s, Dimension, Ix, IxDyn};
use ordered_float::NotNan;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::iter::FromIterator;
use std::str::FromStr;

define_language! {
    pub enum Language {
        // (move-axis <tensor> <axis (usize)> <dest (usize)>)
        // Moves axis <axis> so that it is now axis <dest>.
        // Replaces the "rows" and "cols" operators.
        "move-axis" = MoveAxis([Id; 3]),

        // (cartesian-product <t0> <t1>)
        // Expects tensors of shape
        // [a1, ..., an, c]
        // [b1, ..., bm, c]
        // Outputs a tensor of shape
        // [a1, ..., an, b1, ..., bm, 2, c]
        // which represents the cartesian product of the c-length vectors stored
        // in the two tensors.
        "cartesian-product" = CartesianProduct([Id; 2]),

        // (map-dot-product <tensor>)
        // for a tensor with shape
        // [a1, ..., an, 2, c],
        // the result is a new tensor with shape
        // [a1, ..., an]
        // Whose elements are the dot product of the two c-length vectors at
        // each position in the original array.
        "map-dot-product" = MapDotProduct(Id),

        // (slice <tensor> <axis (usize)> <low (usize)> <high (usize)>)
        // Slices into <tensor> at axis <axis>, slicing the half-open range
        // [<low>, <high>).
        "slice" = Slice([Id; 4]),

        // (concatenate <t0> <t1> <axis (usize)>)
        // Concatenate tensors <t0> and <t1> along <axis>.
        "concatenate" = Concatenate([Id; 3]),


        // (elementwise-add <t0> <t1>)
        // TODO(@gussmith23) this will probably need to be signed at some point?
        // TODO(@gussmith23) ^^ what did I mean by this?
        "elementwise-add" = ElementwiseAdd([Id; 2]),

        // (bsg-systolic-array <rows (usize)> <cols (usize)> <t0> <t1>)
        // Represents a systolic array of size rows X cols, fed with tensors t0
        // and t1.
        // TODO(@gussmith23) do we need to specify rows and cols? You can infer these
        // from the size of the input, but it's also useful for searching.
        "bsg-systolic-array" = BsgSystolicArray([Id; 4]),

        // (systolic-array <rows (usize)> <cols (usize)> <access-0> <access-1>)
        // Represents a systolic array of size rows X cols, fed with two
        // accesses.
        // This is Scott's weight-stationary systolic array design. It reads in
        // two matrices: first matrix is in the layout [M, N] and second is in
        // the layout [N, O]. The systolic array computes the matrix
        // multiplication, leading to a matrix with layout [M, O].
        // The systolic array expects exactly one shape for the second argument:
        // [N, O]. These correspond directly to the rows/cols parameters of the
        // systolic array. The first argument is partially constrained: its
        // second dimension must be N, but its first dimension may be any
        // length.
        // In terms of Glenside accesses, we expect <access-0> to have shape [M]
        // [N], and <access-1> to have shape [] [N, O].
        // TODO(@gussmith23) How to make the M argument "programmable"?
        // TODO(@gussmith23) do we need to specify rows and cols? You can infer these
        // from the size of the input, but it's also useful for searching.
        "systolic-array" = SystolicArray([Id; 4]),

        // Same as the systolic array above, but relies on Scott's blocking code
        // instead of relying on Glenside to discover the blocking. By
        // "blocking", we mean splitting up a matrix multiply to run on a
        // smaller systolic array.
        "systolic-array-with-blocking" = SystolicArrayWithBlocking([Id; 4]),

        // (access-windows <access> <filters-shape: Shape> <stride-shape: Shape>)
        // Form the windows which will be convolved over.
        // TODO(@gussmith23) AccessWindows shouldn't be specific to filters.
        // AccessWindows is used in other contexts too, i.e. pooling.
        "access-windows" = AccessWindows([Id; 3]),

        // (shape-of <tensor>)
        // Returns the shape of the tensor.
        // TODO(@gussmith) Choose between ([Id; 1]) and (Id) and be consistent
        // When describing the arguments of a construct that takes a single Id
        // argument (like shape-of), we can use (Id) or ([Id; 1]). I'm not sure
        // which is better, but I should choose one and be consistent.
        "shape-of" = ShapeOf([Id; 1]),

        // (slice-shape <shape> <dim>)
        // Slices a shape by taking dimensions >= <dim>.
        "slice-shape" = SliceShape([Id; 2]),

        // (shape-insert-axis <shape: Shape> <axis: usize>)
        // Inserts an axis with value 1.
        "shape-insert-axis" = ShapeInsertAxis([Id; 2]),

        // (shape-remove-axis <shape: Shape> <axis: usize>)
        // Removes axis from shape.
        "shape-remove-axis" = ShapeRemoveAxis([Id; 2]),

        // (access <tensor> <dim>)
        // The most basic access pattern.
        // Let <tensor> have dims d0, .., dn.
        // Interprets <tensor> as a shaped list of shape d0, .., d(<dim>-1)
        // whose elements are of shape d<dim>, .., dn.
        "access" = Access([Id; 2]),

        // (access-transpose <a: access> <new-order: list>)
        // Uses numpy.transpose() semantics. Reorders axes in an access.
        // Does not change the access dimension.
        "access-transpose" = AccessTranspose([Id; 2]),

        // (access-cartesian-product <access1> <access2>)
        // Cartesian product access pattern.
        // Assume <access1> has shape
        // [a1, ..., an]
        // and <access2> has shape
        // [b1, ..., bm].
        // Both must have the same item shape,
        // [c1, ..., co]
        // Outputs a tensor of shape
        // [a1, ..., an, b1, ..., bm, 2, c1, ..., co]
        // which represents the cartesian product of the items in both accesses.
        "access-cartesian-product" = AccessCartesianProduct([Id; 2]),

        // (compute <compute-type> <access>)
        // Compute over the items in <access>.
        //
        // Compute types:
        //
        // dot-product
        // Expects an item shape of
        // [n, a0, ..., am]
        // Where n specifies the tuple multiplicity and [a0, ..., am] is the
        // shape of the tensors to be dot-producted with one another.
        "compute" = Compute([Id; 2]),

        // (get-access-shape <access>)
        // Returns the shape of the access.
        // "get-access-shape" = GetAccessShape(Id),
        // This shouldn't actually be needed at the moment. We are handling all
        // statically-sized networks, and so anywhere where we would have used
        // this, we should be able to just plug in a literal access-shape. If
        // and when we start supporting dynamic networks, this will become
        // needed.

        // (access-reshape <access> <shape>)
        // Reshapes the access to have the given
        "access-reshape" = AccessReshape([Id; 2]),

        // (access-flatten <access>)
        // Flattens the access's shape and item shape.
        "access-flatten" = AccessFlatten(Id),

        // (shape <usize>...)
        // Shape literal.
        "shape" = Shape(Box<[Id]>),

        // (list <usize>...)
        // List literal
        "list" = List(Box<[Id]>),

        // (access-shape <shape: shape> <item-shape: shape>)
        // Access shape literal.
        "access-shape" = AccessShape([Id;2]),

        // (access-slice <access> <axis (usize)> <low (usize)> <high (usize)>)
        // Slices into <access> at axis <axis>, slicing the half-open range
        // [<low>, <high>).
        // TODO(@gussmith23) Implement access-slice-item
        // If axis >= access.shape.ndim(), it slices into access.item_shape.
        // This is me being lazy and not wanting to implement separate
        // access-slice-shape and access-slice-item operators for right now.
        "access-slice" = AccessSlice([Id; 4]),

        // (access-concatenate <a0> <a1> <axis (usize)>)
        // Concatenate accesses <a0> and <a1> along <axis>.
        // TODO(@gussmith23) Implement access-concatenate-item
        // If axis >= access.shape.ndim(), it concatenates along dimensions in
        // access.item_shape.
        "access-concatenate" = AccessConcatenate([Id; 3]),

        // (access-pair <a0> <a1>)
        // Simply pair every item of a0 with every item of a1.
        "access-pair" = AccessPair([Id; 2]),

        // (access-shift-right <a0>)
        // Shifts a dimension from shape to item shape.
        "access-shift-right" = AccessShiftRight(Id),

        // (access-tensor <t>)
        // Access a tensor literal.
        "access-tensor" = AccessTensor(Id),

        // (access-pad <a>
        //             <pad-type (PadType)>
        //             <axis (usize)> <pad-before (usize)> <pad-after (usize)>)
        // Pads a tensor at the given axis.
        "access-pad" = AccessPad([Id; 5]),

        // (access-squeeze <a> <axis (usize)>)
        "access-squeeze" = AccessSqueeze([Id; 2]),

        // (access-insert-axis <a> <axis (usize)>)
        "access-insert-axis" = AccessInsertAxis([Id; 2]),

        // (access-broadcast <a> <shape: shape>)
        // Simple broadcasting. <a> and <shape> must have the same total number
        // of dimensions. All dimensions in <a> must either match the
        // corresponding dimension in <shape> or be 1.
        "access-broadcast" = AccessBroadcast([Id; 2]),

        // (access-literal <literal: Literal>)
        // Access a literal. This may be able to be folded in to some other
        // access pattern, later on. It fits in with access-tensor as a "access
        // pattern constructor"; it takes something that isn't an access pattern
        // and converts it to an access pattern.
        "access-literal" = AccessLiteral(Id),

        // (literal <val: Float64>)
        // A literal value. Can only represent 0-dimensional values for now, but
        // in the future, we can and should support array constants.
        "literal" = Literal(Id),


        Usize(usize),

        // Important that this go after usize, so that usizes are parsed as
        // usizes, not as floats.
        NotNanFloat64(NotNan<f64>),

        // pad-type: zero-padding
        // (No other options right now)
        PadType(PadType),

        ComputeType(ComputeType),

        Symbol(String),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ComputeType {
    DotProduct,
    ReduceSum,
    ReLU,
    Sqrt,
    Negative,
    /// Expects item shape of `a x b1 x .. x bn`. Performs an elementwise
    /// addition of the `a` tensors of size `b1 x .. x bn`.
    /// TODO(@gussmith) Multiple-arg compute feels clunky and ad-hoc.
    /// Should figure out an explicit way to define access multiple-stream
    /// access patterns.
    ElementwiseAdd,
    /// Expects item shape of `a x b1 x .. x bn`. Performs an elementwise
    /// multiplication of the `a` tensors of size `b1 x .. x bn`.
    ElementwiseMul,
    ElementwiseDiv,
    /// Takes the max across all elements in each item. Reduces any item shape
    /// to a scalar.
    ReduceMax,
    /// Computes softmax. Currently expects access axis to be 0. Unsure how to
    /// define softmax for other access patterns.
    Softmax,
    /// For an item shape of `a1 x a2 x ...`, returns an item shape of `1` where
    /// the returned scalar is the mean of the `a1 x a2 x ...`-shaped tensor.
    ReduceMean,
}
impl FromStr for ComputeType {
    type Err = ();
    fn from_str(input: &str) -> Result<ComputeType, Self::Err> {
        match input {
            "dot-product" => Ok(ComputeType::DotProduct),
            "reduce-sum" => Ok(ComputeType::ReduceSum),
            "reduce-max" => Ok(ComputeType::ReduceMax),
            "relu" => Ok(ComputeType::ReLU),
            "sqrt" => Ok(ComputeType::Sqrt),
            "negative" => Ok(ComputeType::Negative),
            "elementwise-add" => Ok(ComputeType::ElementwiseAdd),
            "elementwise-mul" => Ok(ComputeType::ElementwiseMul),
            "elementwise-div" => Ok(ComputeType::ElementwiseDiv),
            "softmax" => Ok(ComputeType::Softmax),
            "reduce-mean" => Ok(ComputeType::ReduceMean),
            _ => Err(()),
        }
    }
}
impl Display for ComputeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ComputeType::DotProduct => "dot-product",
                ComputeType::ReduceSum => "reduce-sum",
                ComputeType::ReduceMax => "reduce-max",
                ComputeType::ReLU => "relu",
                ComputeType::Sqrt => "sqrt",
                ComputeType::Negative => "negative",
                ComputeType::ElementwiseAdd => "elementwise-add",
                ComputeType::ElementwiseMul => "elementwise-mul",
                ComputeType::ElementwiseDiv => "elementwise-div",
                ComputeType::Softmax => "softmax",
                ComputeType::ReduceMean => "reduce-mean",
            }
        )
    }
}

/// Specifies how to pick the values we pad with.
#[derive(Debug, PartialEq, Eq, Hash, Clone, PartialOrd, Ord, Copy)]
pub enum PadType {
    /// Pad with zeroes.
    ZeroPadding,
    /// Pad with minimum representable number in the number system.
    MinPadding,
}
impl FromStr for PadType {
    type Err = ();
    fn from_str(input: &str) -> Result<PadType, Self::Err> {
        match input {
            "zero-padding" => Ok(PadType::ZeroPadding),
            "min-padding" => Ok(PadType::MinPadding),
            _ => Err(()),
        }
    }
}
impl Display for PadType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PadType::ZeroPadding => "zero-padding",
                PadType::MinPadding => "min-padding",
            }
        )
    }
}

// TODO(@gussmith23) Pick a better analysis name.
#[derive(Debug, Clone, PartialEq)]
pub enum MyAnalysisData {
    Literal(ndarray::ArrayD<f64>),
    Legacy(MyAnalysisDataLegacyData),
    AccessPattern(AccessPatternData),
    Shape(ShapeData),
    // TODO(@gussmith23) Needed?
    //Tensor(TensorData),
    ComputeType(ComputeType),
    PadType(PadType),
    List(Vec<usize>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapeData {
    shape: IxDyn,
}

/// New version of rangeset.
pub trait RangeSet2 {
    type Index;

    /// Inserts elements, shifting existing ranges as needed.
    fn insert_elements(&mut self, index: Self::Index, num_elements_inserted: usize);

    /// Updates ranges as if `num_elements_removed` elements were removed at
    /// `index`.
    fn remove_elements(&mut self, index: Self::Index, num_elements_removed: usize);

    /// Checks whether `range` is covered by the ranges in this set.
    fn covered(&self, range: (Self::Index, Self::Index)) -> bool;

    /// Adds range. Ranges are half-open.
    fn add_range(&mut self, range: (Self::Index, Self::Index));
}

type BoolVecRangeSet = Vec<bool>;
impl RangeSet2 for BoolVecRangeSet {
    type Index = usize;

    fn insert_elements(&mut self, index: Self::Index, num_elements_inserted: usize) {
        // Make index-1 the last valid index.
        if index >= self.len() {
            self.resize(index, false);
        }
        *self = self[..index]
            .iter()
            .chain(std::iter::repeat(&false).take(num_elements_inserted))
            .chain(self[index..].iter())
            .cloned()
            .collect();
    }

    fn remove_elements(&mut self, index: Self::Index, num_elements_removed: usize) {
        *self = self[..index]
            .iter()
            .chain(self[index + num_elements_removed..].iter())
            .cloned()
            .collect()
    }

    fn covered(&self, range: (Self::Index, Self::Index)) -> bool {
        // If the top end of the range is not actually represented, then those
        // values are implicitly false and so the range is not covered.
        // Otherwise, check that the values are all true.
        range.1 <= self.len() && self[range.0..range.1].iter().all(|v| *v)
    }

    fn add_range(&mut self, range: (Self::Index, Self::Index)) {
        // Make range.1-1 the last valid index.
        if range.1 > self.len() {
            self.resize(range.1, false);
        }
        for i in range.0..range.1 {
            self[i] = true;
        }
    }
}

#[cfg(test)]
mod bool_vec_range_set_tests {
    use super::*;

    #[test]
    fn insert_elements_0() {
        let mut range_set = BoolVecRangeSet::default();
        range_set.add_range((0, 3));
        range_set.add_range((2, 6));
        range_set.add_range((4, 8));
        range_set.add_range((7, 10));
        range_set.insert_elements(5, 5);
        assert!(range_set.covered((0, 3)));
        assert!(range_set.covered((2, 5)));
        assert!(range_set.covered((10, 11)));
        assert!(range_set.covered((4, 5)));
        assert!(range_set.covered((10, 13)));
        assert!(range_set.covered((12, 15)));
    }

    #[test]
    fn insert_elements_1() {
        let mut range_set = BoolVecRangeSet::default();
        range_set.add_range((0, 3));
        range_set.add_range((2, 6));
        range_set.add_range((4, 8));
        range_set.add_range((7, 10));
        range_set.insert_elements(5, 5);
        range_set.add_range((5, 10));
        assert!(range_set.covered((0, 3)));
        assert!(range_set.covered((2, 11)));
        assert!(range_set.covered((4, 13)));
        assert!(range_set.covered((12, 15)));
    }

    #[test]
    fn remove_elements() {
        let mut range_set = BoolVecRangeSet::default();
        range_set.add_range((0, 3));
        range_set.add_range((2, 6));
        range_set.add_range((5, 8));
        range_set.add_range((9, 12));
        range_set.add_range((10, 14));
        range_set.remove_elements(5, 5);
        assert!(range_set.covered((0, 3)));
        assert!(range_set.covered((2, 5)));
        assert!(range_set.covered((5, 7)));
        assert!(range_set.covered((5, 9)));
    }

    #[test]
    fn covered() {
        let mut range_set = BoolVecRangeSet::default();
        range_set.add_range((0, 3));
        range_set.add_range((5, 6));
        range_set.add_range((6, 8));
        range_set.add_range((10, 12));
        range_set.add_range((11, 14));
        assert!(range_set.covered((0, 2)));
        assert!(!range_set.covered((0, 4)));
        assert!(!range_set.covered((2, 5)));
        assert!(!range_set.covered((3, 5)));
        assert!(range_set.covered((5, 7)));
        assert!(range_set.covered((5, 8)));
        assert!(!range_set.covered((5, 9)));
        assert!(range_set.covered((10, 14)));
        assert!(!range_set.covered((10, 16)));
        assert!(!range_set.covered((22, 23)));
    }

    #[test]
    fn test() {
        let mut range_set = BoolVecRangeSet::default();
        range_set.insert_elements(0, 1);
        range_set.add_range((0, 1));
        range_set.insert_elements(33, 2);
        range_set.add_range((33, 35));
        assert!(range_set.covered((0, 1)));
        assert!(!range_set.covered((1, 33)));
        assert!(range_set.covered((33, 35)));
    }
}

/// Used to represent ranges over a set from 0..n, for some n. Ranges are
/// half-open.
type RangeHashSet = HashSet<(usize, usize)>;
pub enum RangeInsertStrategy {
    /// If elements are inserted in the middle of a range, the range gets split
    /// in two.
    BreakRanges,
    /// If elements are inserted in the middle of a range, they get folded into
    /// the range.
    PreserveRanges,
}
pub trait RangeSet {
    type Index;

    /// Updates ranges as if `num_elements_inserted` elements were inserted at
    /// `index`, according to `strategy`.
    fn insert_elements(
        &mut self,
        strategy: RangeInsertStrategy,
        index: Self::Index,
        num_elements_inserted: usize,
    );

    /// Updates ranges as if `num_elements_removed` elements were removed at
    /// `index`.
    fn remove_elements(&mut self, index: Self::Index, num_elements_removed: usize);

    /// Checks whether `range` is covered by the ranges in this set.
    fn covered(&self, range: (Self::Index, Self::Index)) -> bool;

    /// Adds range. Ranges are half-open.
    fn add_range(&mut self, range: (Self::Index, Self::Index));
}
impl RangeSet for RangeHashSet {
    type Index = usize;

    fn insert_elements(
        &mut self,
        strategy: RangeInsertStrategy,
        index: usize,
        num_elements_inserted: usize,
    ) {
        let mut new_ranges = Vec::default();
        for (low, high) in self.drain() {
            assert!(low <= high);
            match strategy {
                RangeInsertStrategy::PreserveRanges => {
                    let (new_low, new_high) = if index < low {
                        (low + num_elements_inserted, high + num_elements_inserted)
                    } else if index >= low && index <= high {
                        (low, high + num_elements_inserted)
                    } else if index > high {
                        (low, high)
                    } else {
                        unreachable!()
                    };
                    new_ranges.push((new_low, new_high));
                }
                RangeInsertStrategy::BreakRanges => {
                    match {
                        if index <= low {
                            (
                                Some((low + num_elements_inserted, high + num_elements_inserted)),
                                None,
                            )
                        } else if index > low && index < high {
                            (
                                Some((low, index)),
                                Some((index + num_elements_inserted, high + num_elements_inserted)),
                            )
                        } else if index >= high {
                            (Some((low, high)), None)
                        } else {
                            unreachable!()
                        }
                    } {
                        (Some(range1), Some(range2)) => {
                            new_ranges.push(range1);
                            new_ranges.push(range2);
                        }
                        (Some(range1), None) => {
                            new_ranges.push(range1);
                        }
                        _ => panic!(),
                    };
                }
            }
        }

        for range in new_ranges.iter() {
            self.insert(*range);
        }
    }

    fn remove_elements(&mut self, index: usize, num_elements_removed: usize) {
        let new_ranges = self
            .drain()
            .filter_map(|(low, high): (usize, usize)| {
                let new_low = if low <= index {
                    low
                } else if low > index {
                    low - std::cmp::min(num_elements_removed, low - index)
                } else {
                    unreachable!()
                };
                let new_high = if index >= high {
                    high
                } else if index < high {
                    high - std::cmp::min(num_elements_removed, high - index)
                } else {
                    unreachable!()
                };

                // If the range is valid and nonempty
                if new_low < new_high {
                    Some((new_low, new_high))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for new_range in new_ranges {
            self.insert(new_range);
        }
    }

    /// I'm hoping this implementation will be fast.
    fn covered(&self, range: (Self::Index, Self::Index)) -> bool {
        let mut to_be_covered =
            HashSet::<_, std::collections::hash_map::RandomState>::from_iter(range.0..range.1);

        for (low, high) in self.iter() {
            to_be_covered = to_be_covered
                .difference(&HashSet::from_iter(*low..*high))
                .cloned()
                .collect();
        }

        to_be_covered.is_empty()
    }

    /// Adds range. Ranges are half-open.
    fn add_range(&mut self, range: (Self::Index, Self::Index)) {
        self.insert(range);
    }
}

#[cfg(test)]
mod range_hash_set_tests {
    use super::*;

    #[test]
    fn insert_elements_break_ranges() {
        let mut range_set = RangeHashSet::default();
        range_set.add_range((0, 3));
        range_set.add_range((2, 6));
        range_set.add_range((4, 8));
        range_set.add_range((7, 10));
        range_set.insert_elements(RangeInsertStrategy::BreakRanges, 5, 5);
        assert_eq!(range_set.len(), 6);
        assert!(range_set.contains(&(0, 3)));
        assert!(range_set.contains(&(2, 5)));
        assert!(range_set.contains(&(10, 11)));
        assert!(range_set.contains(&(4, 5)));
        assert!(range_set.contains(&(10, 13)));
        assert!(range_set.contains(&(12, 15)));
    }

    #[test]
    fn insert_elements_preserve_ranges() {
        let mut range_set = RangeHashSet::default();
        range_set.add_range((0, 3));
        range_set.add_range((2, 6));
        range_set.add_range((4, 8));
        range_set.add_range((7, 10));
        range_set.insert_elements(RangeInsertStrategy::PreserveRanges, 5, 5);
        assert_eq!(range_set.len(), 4);
        assert!(range_set.contains(&(0, 3)));
        assert!(range_set.contains(&(2, 11)));
        assert!(range_set.contains(&(4, 13)));
        assert!(range_set.contains(&(12, 15)));
    }

    #[test]
    fn remove_elements() {
        let mut range_set = RangeHashSet::default();
        range_set.add_range((0, 3));
        range_set.add_range((2, 6));
        range_set.add_range((5, 8));
        range_set.add_range((9, 12));
        range_set.add_range((10, 14));
        range_set.remove_elements(5, 5);
        assert_eq!(range_set.len(), 4);
        assert!(range_set.contains(&(0, 3)));
        assert!(range_set.contains(&(2, 5)));
        assert!(range_set.contains(&(5, 7)));
        assert!(range_set.contains(&(5, 9)));
    }

    #[test]
    fn covered() {
        let mut range_set = RangeHashSet::default();
        range_set.add_range((0, 3));
        range_set.add_range((5, 6));
        range_set.add_range((6, 8));
        range_set.add_range((10, 12));
        range_set.add_range((11, 14));
        assert!(range_set.covered((0, 2)));
        assert!(!range_set.covered((0, 4)));
        assert!(!range_set.covered((2, 5)));
        assert!(!range_set.covered((3, 5)));
        assert!(range_set.covered((5, 7)));
        assert!(range_set.covered((5, 8)));
        assert!(!range_set.covered((5, 9)));
        assert!(range_set.covered((10, 14)));
        assert!(!range_set.covered((10, 16)));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccessPatternData {
    pub shape: IxDyn,
    pub item_shape: IxDyn,
    /// Regions proven to be zero-valued. The outermost map maps a axis index to
    /// a set of usize tuples, indicating the half-open indices [low, high)
    /// which are known to be zero in that axis.
    /// TODO(@gussmith23) Might want to replace this with full partial eval
    /// I realized while implementing this that I could just implement partial
    /// eval, and it would do the same for me. I don't think it would be more
    /// time efficient, though, and I'm even more certain that it wouldn't be
    /// space efficient.
    pub zero_regions: HashMap<Ix, BoolVecRangeSet>,
}

impl AccessPatternData {
    /// Convenience method for getting the access pattern dimensions as a
    /// vector.
    /// ```
    /// assert_eq!(
    ///     glenside::language::AccessPatternData {
    ///         shape: ndarray::IxDyn(&[1, 2, 3]),
    ///         item_shape: ndarray::IxDyn(&[4, 5]),
    ///         zero_regions: std::collections::HashMap::default()
    ///     }
    ///     .as_vec(),
    ///     vec![1, 2, 3, 4, 5]
    /// );
    /// ```
    pub fn as_vec(&self) -> Vec<usize> {
        self.shape
            .slice()
            .iter()
            .chain(self.item_shape.slice().iter())
            .cloned()
            .collect::<Vec<_>>()
    }
}

impl std::ops::Index<usize> for AccessPatternData {
    type Output = ndarray::Ix;
    fn index(&self, index: usize) -> &Self::Output {
        if index < self.shape.ndim() {
            &self.shape[index]
        } else {
            &self.item_shape[index - self.shape.ndim()]
        }
    }
}

pub fn access_windows_resulting_shape(
    access_shape: &IxDyn,
    filters_shape: &IxDyn,
    stride_shape: &IxDyn,
) -> Vec<usize> {
    assert_eq!(access_shape.ndim(), stride_shape.ndim());
    assert_eq!(filters_shape.ndim(), stride_shape.ndim());

    multizip((
        access_shape.slice().iter(),
        filters_shape.slice().iter(),
        stride_shape.slice().iter(),
    ))
    .map(
        |(&dim_len, &kernel_dim_len, &stride): (&usize, &usize, &usize)| {
            let total_dim_len = dim_len;
            assert!(total_dim_len >= kernel_dim_len);
            let num_spots = total_dim_len - (kernel_dim_len - 1);
            (num_spots + stride - 1) / stride
        },
    )
    .collect()
}

// #[derive(Debug, Clone, PartialEq)]
// pub struct TensorData {
//     shape: IxDyn,
// }

// TODO(@gussmith23) Pick a better analysis name.
#[derive(Debug, Clone, PartialEq)]
pub struct MyAnalysisDataLegacyData {
    pub(crate) shape: Option<IxDyn>,
    pub(crate) usize_value: Option<usize>,
}
#[derive(Default)]
pub struct MyAnalysis {
    pub name_to_shape: HashMap<String, Vec<usize>>,
}
impl MyAnalysis {
    pub fn get_usize(id: Id, egraph: &EGraph<Language, MyAnalysis>) -> usize {
        match &egraph[id].data {
            MyAnalysisData::Legacy(s) => s.usize_value.unwrap(),
            _ => panic!(),
        }
    }
    pub(crate) fn get_shape(id: Id, egraph: &EGraph<Language, MyAnalysis>) -> &IxDyn {
        match &egraph[id].data {
            MyAnalysisData::Legacy(s) => s.shape.as_ref().unwrap(),
            _ => panic!(),
        }
    }
    pub(crate) fn get_shape_of_value(id: Id, egraph: &EGraph<Language, MyAnalysis>) -> &IxDyn {
        match &egraph[id].data {
            MyAnalysisData::Shape(s) => &s.shape,
            _ => panic!(),
        }
    }
}
impl egg::Analysis<Language> for MyAnalysis {
    type Data = MyAnalysisData;

    fn merge(&self, to: &mut Self::Data, from: Self::Data) -> bool {
        match (to, &from) {
            (
                MyAnalysisData::AccessPattern(AccessPatternData {
                    shape: to_shape,
                    item_shape: to_item_shape,
                    zero_regions: to_zero_regions,
                }),
                MyAnalysisData::AccessPattern(AccessPatternData {
                    shape: from_shape,
                    item_shape: from_item_shape,
                    zero_regions: from_zero_regions,
                }),
            ) => {
                assert_eq!(to_shape, from_shape);
                assert_eq!(to_item_shape, from_item_shape);

                // Merge zero regions.
                // TODO(@gussmith23) Make sure merge returns `true` infrequently
                // Returning `true` more often forces more rebuilds, which kills
                // performance!
                let mut changed = false;
                for (axis_index, from_range_set) in from_zero_regions.iter() {
                    // Skip if `from` doesn't contain any interesting data.
                    if !from_range_set.iter().any(|v| *v) {
                        continue;
                    }

                    if let Some(to_range_set) = to_zero_regions.get_mut(&axis_index) {
                        // We first check whether `from_zero_regions` contains
                        // any information not already known in
                        // `to_zero_regions`. This is done by checking them
                        // element-by-element. If it is ever true that
                        // `from_zero_regions` contains a `true` where
                        // `to_zero_regions` contains a `false` or does not have
                        // data (because they may be different lengths), then
                        // they're different and must be merged.

                        // TODO(@gussmith23) Delete these
                        //println!("to: {:?}", to_range_set.len());
                        //println!("from: {:?}", from_range_set.len());

                        // Check.
                        let needs_merge = to_range_set
                            .iter()
                            .zip_longest(from_range_set.iter())
                            .map(|v| {
                                match v {
                                    // `*from` being true implies `*to` must be true.
                                    Both(to, from) => {
                                        if *from {
                                            *from != *to
                                        } else {
                                            false
                                        }
                                    }
                                    // If `to` has a value and `from` doesn't, then
                                    // no merging needed.
                                    Left(_) => false,
                                    // If `from` has a value, then we need to merge
                                    // if that value is true.
                                    Right(from) => *from,
                                }
                            })
                            .any(|v| v);

                        if needs_merge {
                            *to_range_set = to_range_set
                                .iter()
                                .zip_longest(from_range_set.iter())
                                .map(|v| match v {
                                    Both(to, from) => *to || *from,
                                    Left(to) => *to,
                                    Right(from) => *from,
                                })
                                .collect();
                            changed = true;
                        }
                    } else {
                        // If no info exists for this axis in `to_zero_regions`,
                        // then we insert the information from
                        // `from_zero_regions`, but only if there's actual
                        // useful information there (i.e. at least one `true`
                        // value).
                        if from_range_set.iter().any(|v| *v) {
                            to_zero_regions.insert(*axis_index, from_range_set.clone());
                            changed = true;
                        }
                    }
                }

                changed
            }
            (to @ _, _) => {
                assert_eq!(*to, from);
                merge_if_different(to, from)
            }
        }
    }

    fn make(egraph: &EGraph<Language, Self>, enode: &Language) -> Self::Data {
        use Language::*;
        match enode {
            &AccessLiteral(id) => match &egraph[id].data {
                MyAnalysisData::Literal(t) => MyAnalysisData::AccessPattern(AccessPatternData {
                    zero_regions: {
                        warn!("Zero regions unimplemented on line {}", std::line!());
                        HashMap::default()
                    },
                    shape: IxDyn(&[]),
                    item_shape: IxDyn(t.shape()),
                }),
                _ => panic!(),
            },
            &NotNanFloat64(v) => MyAnalysisData::Literal(ndarray::arr0(v.into_inner()).into_dyn()),
            &Literal(id) => match &egraph[id].data {
                t @ MyAnalysisData::Literal(_) => t.clone(),
                _ => panic!(),
            },
            &AccessTranspose([access_id, list_id]) => {
                let access = match &egraph[access_id].data {
                    MyAnalysisData::AccessPattern(a) => a,
                    _ => panic!(),
                };
                let list = match &egraph[list_id].data {
                    MyAnalysisData::List(l) => l,
                    _ => panic!(),
                };

                assert_eq!(
                    access.shape.ndim() + access.item_shape.ndim(),
                    list.len(),
                    "Number of items in list should equal the number of axes in the first argument"
                );
                let tmp = access
                    .shape
                    .slice()
                    .iter()
                    .chain(access.item_shape.slice().iter())
                    .collect::<Vec<_>>();
                let new_shape = list.iter().map(|i| *tmp[*i]).collect::<Vec<_>>();

                // Re-sort zero regions.
                let mut new_zero_regions = HashMap::default();
                for (new_axis_index, old_axis_index) in list.iter().enumerate() {
                    if let Some(val) = access.zero_regions.get(old_axis_index) {
                        new_zero_regions.insert(new_axis_index, val.clone());
                    }
                }

                MyAnalysisData::AccessPattern(AccessPatternData {
                    shape: IxDyn(&new_shape[..access.shape.ndim()]),
                    item_shape: IxDyn(&new_shape[access.shape.ndim()..]),
                    zero_regions: new_zero_regions,
                })
            }
            List(list) => {
                let list = list
                    .iter()
                    .map(|id| MyAnalysis::get_usize(*id, egraph))
                    .collect::<Vec<_>>();
                MyAnalysisData::List(list)
            }
            &AccessBroadcast([access_id, shape_id]) => {
                let access = match &egraph[access_id].data {
                    MyAnalysisData::AccessPattern(a) => a,
                    _ => panic!(),
                };
                let shape = match &egraph[shape_id].data {
                    MyAnalysisData::AccessPattern(a) => a,
                    _ => panic!(
                        "Expected access shape as second argument of access-broadcast, got {:?}",
                        egraph[shape_id]
                    ),
                };

                assert_eq!(
                    access.shape.ndim() + access.item_shape.ndim(),
                    shape.shape.ndim() + shape.item_shape.ndim(),
                    "Shape we're broadcasting to should have the same number of dimensions as the shape we're broadcasting from"
                );

                let new_shape = access
                    .shape
                    .slice()
                    .iter()
                    .chain(access.item_shape.slice().iter())
                    .zip(
                        shape
                            .shape
                            .slice()
                            .iter()
                            .chain(shape.item_shape.slice().iter()),
                    )
                    .map(|(broadcast_from_dim, broadcast_to_dim): (&usize, &usize)| {
                        assert!(
                            *broadcast_from_dim == 1 || broadcast_from_dim == broadcast_to_dim,
                            "Expected broadcast_from_dim to be 1 or {}, got {}",
                            *broadcast_to_dim,
                            *broadcast_from_dim
                        );
                        *broadcast_to_dim
                    })
                    .collect::<Vec<_>>();

                if !access.zero_regions.is_empty() {
                    warn!(
                        "Throwing away zero region analysis data on line {}",
                        std::line!()
                    );
                }

                assert_eq!(
                    new_shape.len(),
                    access.shape.ndim() + access.item_shape.ndim()
                );

                MyAnalysisData::AccessPattern(AccessPatternData {
                    shape: IxDyn(&new_shape[..access.shape.ndim()]),
                    item_shape: IxDyn(&new_shape[access.shape.ndim()..]),
                    // TODO(@gussmith23) Implement zero regions
                    // It's harmless (I think) if `zero_regions` defaults to
                    // empty, but for it to be useful, we need to implement it
                    // for each operator.
                    zero_regions: {
                        if !access.zero_regions.is_empty() {
                            warn!(
                                "Throwing away zero region analysis data on line {}",
                                std::line!()
                            );
                        }
                        HashMap::default()
                    },
                })
            }
            &AccessInsertAxis([access_id, axis_id]) => {
                let mut access = match &egraph[access_id].data {
                    MyAnalysisData::AccessPattern(a) => a.clone(),
                    _ => panic!(),
                };
                // TODO(@gussmith23) Implement zero_regions
                if !access.zero_regions.is_empty() {
                    warn!(
                        "Throwing away zero region analysis data on line {}",
                        std::line!()
                    );
                    access.zero_regions = HashMap::default();
                }
                let axis = MyAnalysis::get_usize(axis_id, egraph);

                assert!(axis <= access.shape.ndim() + access.item_shape.ndim());

                if axis <= access.shape.ndim() {
                    access.shape = IxDyn(
                        access.shape.slice()[..axis]
                            .iter()
                            .cloned()
                            .chain(std::iter::once(1))
                            .chain(access.shape.slice()[axis..].iter().cloned())
                            .collect::<Vec<_>>()
                            .as_slice(),
                    );
                } else {
                    let n = access.shape.ndim();
                    access.item_shape = IxDyn(
                        access.item_shape.slice()[..axis - n]
                            .iter()
                            .cloned()
                            .chain(std::iter::once(1))
                            .chain(access.item_shape.slice()[axis - n..].iter().cloned())
                            .collect::<Vec<_>>()
                            .as_slice(),
                    );
                }

                MyAnalysisData::AccessPattern(access)
            }
            &AccessSqueeze([access_id, axis_id]) => {
                let mut access = match &egraph[access_id].data {
                    MyAnalysisData::AccessPattern(a) => a.clone(),
                    _ => panic!(),
                };
                // TODO(@gussmith23) Implement zero_regions
                if !access.zero_regions.is_empty() {
                    warn!(
                        "Throwing away zero region analysis data on line {}",
                        std::line!()
                    );
                    access.zero_regions = HashMap::default();
                }
                let axis = MyAnalysis::get_usize(axis_id, egraph);
                use ndarray::RemoveAxis;
                if axis < access.shape.ndim() {
                    assert_eq!(
                        access.shape[axis], 1,
                        "Expected axis {} of {:?} to be 1",
                        axis, access.shape
                    );
                    access.shape = access.shape.remove_axis(ndarray::Axis(axis));
                } else {
                    assert_eq!(access.item_shape[axis - access.shape.ndim()], 1);
                    access.item_shape = access
                        .item_shape
                        .remove_axis(ndarray::Axis(axis - access.shape.ndim()));
                }

                MyAnalysisData::AccessPattern(access)
            }
            &AccessPad([access_id, pad_type_id, axis_id, pad_before_id, pad_after_id]) => {
                let mut access = match &egraph[access_id].data {
                    MyAnalysisData::AccessPattern(a) => a.clone(),
                    _ => panic!(),
                };
                let pad_type = match &egraph[pad_type_id].data {
                    MyAnalysisData::PadType(t) => t,
                    _ => panic!(),
                };
                let axis = MyAnalysis::get_usize(axis_id, egraph);
                assert!(axis < access.shape.ndim() + access.item_shape.ndim());
                let orig_axis_val = access[axis];
                let pad_before = MyAnalysis::get_usize(pad_before_id, egraph);
                let pad_after = MyAnalysis::get_usize(pad_after_id, egraph);
                if axis < access.shape.ndim() {
                    access.shape[axis] += pad_before + pad_after;
                } else {
                    access.item_shape[axis - access.shape.ndim()] += pad_before + pad_after;
                };

                // TODO(@gussmith23) Remove this after figuring out padding issues
                for (axis, val) in &access.zero_regions {
                    assert!(
                        val.len() <= access[*axis],
                        "{} > {}",
                        val.len(),
                        access[*axis]
                    );
                }

                // Update zero regions
                match pad_type {
                    crate::language::PadType::MinPadding => {
                        if !access.zero_regions.is_empty() {
                            warn!(
                                "Throwing away zero region analysis data on line {}",
                                std::line!()
                            );
                            access.zero_regions = HashMap::default();
                        }
                    }
                    crate::language::PadType::ZeroPadding => {
                        if !access.zero_regions.contains_key(&axis) {
                            access.zero_regions.insert(axis, BoolVecRangeSet::default());
                        }
                        // Update the zero regions. Order here is important (we
                        // do the end padding first, then the beginning)
                        // TODO(@gussmith23) Written in a rush.
                        access
                            .zero_regions
                            .get_mut(&axis)
                            .unwrap()
                            .insert_elements(orig_axis_val, pad_after);
                        access
                            .zero_regions
                            .get_mut(&axis)
                            .unwrap()
                            .add_range((orig_axis_val, orig_axis_val + pad_after));
                        access
                            .zero_regions
                            .get_mut(&axis)
                            .unwrap()
                            .insert_elements(0, pad_before);
                        access
                            .zero_regions
                            .get_mut(&axis)
                            .unwrap()
                            .add_range((0, pad_before));
                    }
                }

                // TODO(@gussmith23) Remove this after figuring out padding issues
                for (axis, val) in &access.zero_regions {
                    assert!(val.len() <= access[*axis]);
                }

                MyAnalysisData::AccessPattern(access)
            }
            &AccessTensor(t_id) => MyAnalysisData::AccessPattern(AccessPatternData {
                // TODO(@gussmith23) Implement zero regions
                // It's harmless (I think) if `zero_regions` defaults to
                // empty, but for it to be useful, we need to implement it
                // for each operator.
                zero_regions: { HashMap::default() },
                shape: match &egraph[t_id].data {
                    MyAnalysisData::Legacy(l) => l.shape.as_ref().unwrap().clone(),
                    _ => panic!(),
                },
                item_shape: IxDyn(&[]),
            }),
            &AccessShiftRight(a_id) => {
                let a = match &egraph[a_id].data {
                    MyAnalysisData::AccessPattern(a) => a,
                    _ => panic!(),
                };

                let combined = a
                    .shape
                    .as_array_view()
                    .iter()
                    .chain(a.item_shape.as_array_view().iter())
                    .cloned()
                    .collect::<Vec<_>>();
                MyAnalysisData::AccessPattern(AccessPatternData {
                    // TODO(@gussmith23) Implement zero regions
                    // It's harmless (I think) if `zero_regions` defaults to
                    // empty, but for it to be useful, we need to implement it
                    // for each operator.
                    zero_regions: {
                        if !a.zero_regions.is_empty() {
                            warn!(
                                "Throwing away zero region analysis data on line {}",
                                std::line!()
                            );
                        }
                        HashMap::default()
                    },
                    shape: IxDyn(&combined[..(a.shape.ndim().saturating_sub(1))]),
                    item_shape: IxDyn(&combined[(a.shape.ndim().saturating_sub(1))..]),
                })
            }
            &AccessPair([a0_id, a1_id]) => {
                let (a0, a1) = match (&egraph[a0_id].data, &egraph[a1_id].data) {
                    (MyAnalysisData::AccessPattern(a0), MyAnalysisData::AccessPattern(a1)) => {
                        (a0, a1)
                    }
                    _ => panic!(),
                };

                assert_eq!(a0.shape, a1.shape);
                assert_eq!(a0.item_shape, a1.item_shape);

                MyAnalysisData::AccessPattern(AccessPatternData {
                    // TODO(@gussmith23) Implement zero regions
                    // It's harmless (I think) if `zero_regions` defaults to
                    // empty, but for it to be useful, we need to implement it
                    // for each operator.
                    zero_regions: {
                        if !a0.zero_regions.is_empty() {
                            warn!(
                                "Throwing away zero region analysis data on line {}",
                                std::line!()
                            );
                        }
                        if !a1.zero_regions.is_empty() {
                            warn!(
                                "Throwing away zero region analysis data on line {}",
                                std::line!()
                            );
                        }
                        HashMap::default()
                    },
                    shape: a0.shape.clone(),
                    item_shape: IxDyn(
                        std::iter::once(2)
                            .chain(a0.item_shape.as_array_view().iter().cloned())
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ),
                })
            }
            &AccessSlice([access_id, axis_id, low_id, high_id]) => {
                let mut new_access = match &egraph[access_id].data {
                    MyAnalysisData::AccessPattern(a) => a.clone(),
                    _ => panic!(),
                };
                let axis: usize = Self::get_usize(axis_id, egraph);
                let low: usize = Self::get_usize(low_id, egraph);
                let high: usize = Self::get_usize(high_id, egraph);
                let original_axis_value = new_access[axis];

                assert!(new_access.shape.ndim() + new_access.item_shape.ndim() > axis);
                if axis < new_access.shape.ndim() {
                    assert!(low < new_access.shape[axis]);
                    assert!(high <= new_access.shape[axis]);
                    new_access.shape[axis] = high - low;
                } else {
                    assert!(low < new_access.item_shape[axis - new_access.shape.ndim()]);
                    assert!(high <= new_access.item_shape[axis - new_access.shape.ndim()]);
                    new_access.item_shape[axis - new_access.shape.ndim()] = high - low;
                }

                // Update zero regions
                if let Some(range_set) = new_access.zero_regions.get_mut(&axis) {
                    // TODO(@gussmith23) should really just have an "envelope"
                    range_set.remove_elements(high, original_axis_value - high);
                    range_set.remove_elements(0, low - 0);
                }

                MyAnalysisData::AccessPattern(new_access)
            }
            &AccessConcatenate([a0_id, a1_id, axis_id]) => {
                let axis = Self::get_usize(axis_id, egraph);
                let mut new_access = match &egraph[a0_id].data {
                    MyAnalysisData::AccessPattern(a) => a.clone(),
                    _ => panic!(),
                };
                let a1 = match &egraph[a1_id].data {
                    MyAnalysisData::AccessPattern(a) => a,
                    _ => panic!(),
                };
                // TODO(@gussmith23) Implement zero_regions
                if !new_access.zero_regions.is_empty() {
                    warn!(
                        "Throwing away zero region analysis data on line {}",
                        std::line!()
                    );
                    new_access.zero_regions = HashMap::default();
                }
                if !a1.zero_regions.is_empty() {
                    warn!(
                        "Throwing away zero region analysis data on line {}",
                        std::line!()
                    );
                }
                assert_eq!(new_access.shape.ndim(), a1.shape.ndim(),);
                assert_eq!(new_access.item_shape.ndim(), a1.item_shape.ndim(),);
                assert!(axis < a1.shape.ndim() + a1.item_shape.ndim());
                if axis < new_access.shape.ndim() {
                    new_access.shape[axis] += a1.shape[axis];
                } else {
                    new_access.item_shape[axis - new_access.shape.ndim()] +=
                        a1.item_shape[axis - new_access.shape.ndim()];
                }

                MyAnalysisData::AccessPattern(new_access)
            }
            &AccessShape([shape_id, item_shape_id]) => {
                MyAnalysisData::AccessPattern(AccessPatternData {
                    zero_regions: { HashMap::default() },
                    shape: match &egraph[shape_id].data {
                        MyAnalysisData::Shape(s) => s.shape.clone(),
                        _ => panic!(),
                    },
                    item_shape: match &egraph[item_shape_id].data {
                        MyAnalysisData::Shape(s) => s.shape.clone(),
                        _ => panic!(),
                    },
                })
            }
            Shape(list) => MyAnalysisData::Shape(ShapeData {
                shape: IxDyn(
                    list.iter()
                        .map(|id: &Id| MyAnalysis::get_usize(*id, egraph))
                        .collect::<Vec<_>>()
                        .as_slice(),
                ),
            }),
            &AccessReshape([access_id, access_shape_id]) => {
                let a = match &egraph[access_id].data {
                    MyAnalysisData::AccessPattern(a) => a,
                    _ => panic!("Expected an access as the first argument to access-reshape"),
                };
                let mut new_shape = match &egraph[access_shape_id].data {
                    MyAnalysisData::AccessPattern(a) => a.clone(),
                    _ => panic!(),
                };
                // TODO(@gussmith23) Implement zero_regions
                new_shape.zero_regions = HashMap::default();
                if !a.zero_regions.is_empty() {
                    warn!(
                        "Throwing away zero region analysis data on line {}",
                        std::line!()
                    );
                }
                assert_eq!(
                    a.shape.as_array_view().iter().product::<usize>(),
                    new_shape.shape.as_array_view().iter().product::<usize>(),
                );
                assert_eq!(
                    a.item_shape.as_array_view().iter().product::<usize>(),
                    new_shape
                        .item_shape
                        .as_array_view()
                        .iter()
                        .product::<usize>(),
                );
                MyAnalysisData::AccessPattern(new_shape)
            }
            &AccessFlatten(access_id) => {
                let a = match &egraph[access_id].data {
                    MyAnalysisData::AccessPattern(a) => a,
                    _ => panic!(),
                };
                MyAnalysisData::AccessPattern(AccessPatternData {
                    // TODO(@gussmith23) Implement zero regions
                    // It's harmless (I think) if `zero_regions` defaults to
                    // empty, but for it to be useful, we need to implement it
                    // for each operator.
                    zero_regions: {
                        if !a.zero_regions.is_empty() {
                            warn!(
                                "Throwing away zero region analysis data on line {}",
                                std::line!()
                            );
                        }
                        HashMap::default()
                    },
                    shape: IxDyn(&[a.shape.as_array_view().iter().product()]),
                    item_shape: IxDyn(&[a.item_shape.as_array_view().iter().product()]),
                })
            }
            ComputeType(t) => MyAnalysisData::ComputeType(t.clone()),
            &Compute([compute_type_id, access_id]) => {
                let compute_type = match &egraph[compute_type_id].data {
                    MyAnalysisData::ComputeType(t) => t,
                    _ => panic!("Argument 0 of {:?} should be a ComputeType", enode),
                };
                let a0 = match &egraph[access_id].data {
                    MyAnalysisData::AccessPattern(a0) => a0,
                    _ => panic!(),
                };
                // TODO(@gussmith23) Implement zero_regions
                if !a0.zero_regions.is_empty() {
                    warn!(
                        "Throwing away zero region analysis data on line {}",
                        std::line!()
                    );
                }

                match compute_type {
                    self::ComputeType::ReduceMean => {
                        MyAnalysisData::AccessPattern(AccessPatternData {
                            // TODO(@gussmith23) Implement zero regions
                            // It's harmless (I think) if `zero_regions` defaults to
                            // empty, but for it to be useful, we need to implement it
                            // for each operator.
                            zero_regions: {
                                if !a0.zero_regions.is_empty() {
                                    warn!(
                                        "Throwing away zero region analysis data on line {}",
                                        std::line!()
                                    );
                                }
                                HashMap::default()
                            },
                            shape: a0.shape.clone(),
                            item_shape: ndarray::IxDyn(&[]),
                        })
                    }
                    self::ComputeType::Softmax => {
                        assert_eq!(
                            a0.item_shape.ndim(),
                            1,
                            "Softmax is only implemented for axis=-1"
                        );
                        MyAnalysisData::AccessPattern(AccessPatternData {
                            // TODO(@gussmith23) Implement zero regions
                            // It's harmless (I think) if `zero_regions` defaults to
                            // empty, but for it to be useful, we need to implement it
                            // for each operator.
                            zero_regions: {
                                if !a0.zero_regions.is_empty() {
                                    warn!(
                                        "Throwing away zero region analysis data on line {}",
                                        std::line!()
                                    );
                                }
                                HashMap::default()
                            },
                            shape: a0.shape.clone(),
                            item_shape: a0.item_shape.clone(),
                        })
                    }
                    self::ComputeType::ElementwiseAdd
                    | self::ComputeType::ElementwiseMul
                    | self::ComputeType::ElementwiseDiv => {
                        assert!(a0.item_shape.ndim() >= 1);
                        MyAnalysisData::AccessPattern(AccessPatternData {
                            // TODO(@gussmith23) Implement zero regions
                            // It's harmless (I think) if `zero_regions` defaults to
                            // empty, but for it to be useful, we need to implement it
                            // for each operator.
                            zero_regions: {
                                if !a0.zero_regions.is_empty() {
                                    warn!(
                                        "Throwing away zero region analysis data on line {}",
                                        std::line!()
                                    );
                                }
                                HashMap::default()
                            },
                            shape: a0.shape.clone(),
                            item_shape: IxDyn(&a0.item_shape.slice()[1..]),
                        })
                    }
                    self::ComputeType::DotProduct => {
                        // If it's =1, that's just a "dot product" of scalars,
                        // which is just a sum.
                        //
                        // Honestly, it could also be 0. It doesn't make much
                        // sense but it's not wrong. Can remove this later if we
                        // want those semantics.
                        assert!(a0.item_shape.ndim() >= 1);

                        // MyAnalysisData::Tensor(TensorData {
                        //     shape: a0.shape.clone(),
                        // })
                        MyAnalysisData::AccessPattern(AccessPatternData {
                            // TODO(@gussmith23) Implement zero regions
                            // It's harmless (I think) if `zero_regions` defaults to
                            // empty, but for it to be useful, we need to implement it
                            // for each operator.
                            zero_regions: {
                                if !a0.zero_regions.is_empty() {
                                    warn!(
                                        "Throwing away zero region analysis data on line {}",
                                        std::line!()
                                    );
                                }
                                HashMap::default()
                            },
                            shape: a0.shape.clone(),
                            item_shape: IxDyn(&[]),
                        })
                    }
                    self::ComputeType::ReduceSum | self::ComputeType::ReduceMax => {
                        MyAnalysisData::AccessPattern(AccessPatternData {
                            // TODO(@gussmith23) Implement zero regions
                            // It's harmless (I think) if `zero_regions` defaults to
                            // empty, but for it to be useful, we need to implement it
                            // for each operator.
                            zero_regions: {
                                if !a0.zero_regions.is_empty() {
                                    warn!(
                                        "Throwing away zero region analysis data on line {}",
                                        std::line!()
                                    );
                                }
                                HashMap::default()
                            },
                            shape: a0.shape.clone(),
                            item_shape: IxDyn(&[]),
                        })
                    }
                    self::ComputeType::ReLU
                    | self::ComputeType::Sqrt
                    | self::ComputeType::Negative => {
                        // TODO(@gussmith23) Implement zero_regions
                        if !a0.zero_regions.is_empty() {
                            warn!(
                                "Throwing away zero region analysis data on line {}",
                                std::line!()
                            );
                        }
                        let mut a = a0.clone();
                        a.zero_regions = HashMap::default();
                        MyAnalysisData::AccessPattern(a)
                    }
                }
            }
            &AccessCartesianProduct([a0_id, a1_id]) => {
                let (a0, a1) = match (&egraph[a0_id].data, &egraph[a1_id].data) {
                    (MyAnalysisData::AccessPattern(a0), MyAnalysisData::AccessPattern(a1)) => {
                        (a0, a1)
                    }
                    _ => panic!(),
                };
                assert_eq!(
                    a0.item_shape, a1.item_shape,
                    "Cartesian product argument shapes must match"
                );

                let new_shape = IxDyn(
                    a0.shape
                        .as_array_view()
                        .iter()
                        .cloned()
                        .chain(a1.shape.as_array_view().iter().cloned())
                        .collect::<Vec<usize>>()
                        .as_slice(),
                );
                let new_item_shape = IxDyn(
                    std::iter::once(2)
                        .chain(a0.item_shape.as_array_view().iter().cloned())
                        .collect::<Vec<usize>>()
                        .as_slice(),
                );

                assert_eq!(
                    new_shape.as_array_view().iter().product::<usize>()
                        * new_item_shape.as_array_view().iter().product::<usize>(),
                    a0.shape.as_array_view().iter().product::<usize>()
                        * a1.shape.as_array_view().iter().product::<usize>()
                        * 2
                        * a0.item_shape.as_array_view().iter().product::<usize>()
                );

                MyAnalysisData::AccessPattern(AccessPatternData {
                    zero_regions: {
                        // TODO(@gussmith23) We only implement zero regions for
                        // item dimensions.
                        // That's all we need for now w/r/t cart prods.

                        let mut zero_regions = HashMap::new();
                        for item_dim in 0..a0.item_shape.ndim() {
                            if let (Some(range_set_0), Some(range_set_1)) = (
                                a0.zero_regions.get(&(a0.shape.ndim() + item_dim)),
                                a1.zero_regions.get(&(a1.shape.ndim() + item_dim)),
                            ) {
                                // Basically, we know a range [:, :, :, :, x] is
                                // filled with zeros if its original ranges [:,
                                // :, x] and [:, :, x] are zeros.
                                let new_range_set: BoolVecRangeSet = range_set_0
                                    .iter()
                                    .zip(range_set_1.iter())
                                    .map(|(v0, v1): (&bool, &bool)| *v0 && *v1)
                                    .collect();
                                if new_range_set.iter().any(|v| *v) {
                                    zero_regions.insert(
                                        a0.shape.ndim() + a1.shape.ndim() + 1 + item_dim,
                                        new_range_set,
                                    );
                                }
                            }
                        }

                        zero_regions
                    },
                    shape: new_shape,
                    item_shape: new_item_shape,
                })
            }
            &SliceShape([shape_id, dim_id]) => {
                let shape = MyAnalysis::get_shape_of_value(shape_id, egraph);
                let dim = MyAnalysis::get_usize(dim_id, egraph);
                MyAnalysisData::Shape(ShapeData {
                    shape: IxDyn(shape.as_array_view().slice(s![dim..]).to_slice().unwrap()),
                })
            }
            &ShapeInsertAxis([shape_id, dim_id]) => {
                let shape = MyAnalysis::get_shape_of_value(shape_id, egraph);
                let dim = MyAnalysis::get_usize(dim_id, egraph);
                assert!(
                    dim <= shape.ndim(),
                    "Invalid dimension {} for shape {:?}",
                    dim,
                    shape
                );
                MyAnalysisData::Shape(ShapeData {
                    shape: IxDyn(
                        shape.slice()[..dim]
                            .iter()
                            .chain(std::iter::once(&1))
                            .chain(shape.slice()[dim..].iter())
                            .cloned()
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ),
                })
            }
            &ShapeRemoveAxis([shape_id, dim_id]) => {
                let shape = MyAnalysis::get_shape_of_value(shape_id, egraph);
                let dim = MyAnalysis::get_usize(dim_id, egraph);
                assert!(
                    dim < shape.ndim(),
                    "Invalid dimension {} for shape {:?}",
                    dim,
                    shape
                );
                MyAnalysisData::Shape(ShapeData {
                    shape: IxDyn(
                        shape.slice()[..dim]
                            .iter()
                            .chain(shape.slice()[dim + 1..].iter())
                            .cloned()
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ),
                })
            }
            &Access([tensor_or_access_id, dim_id]) => {
                // TODO(@gussmith23) How to access tensor literals?
                let dim = MyAnalysis::get_usize(dim_id, egraph);
                let access = match &egraph[tensor_or_access_id].data {
                    MyAnalysisData::AccessPattern(a) => a,
                    _ => panic!(),
                };
                let shape = access
                    .shape
                    .as_array_view()
                    .iter()
                    .chain(access.item_shape.as_array_view().iter())
                    .cloned()
                    .collect::<Vec<_>>();
                MyAnalysisData::AccessPattern(AccessPatternData {
                    // TODO(@gussmith23) Implement zero regions
                    // It's harmless (I think) if `zero_regions` defaults to
                    // empty, but for it to be useful, we need to implement it
                    // for each operator.
                    zero_regions: {
                        if !access.zero_regions.is_empty() {
                            warn!(
                                "Throwing away zero region analysis data on line {}",
                                std::line!()
                            );
                        }
                        HashMap::default()
                    },
                    shape: IxDyn(&shape[..dim]),
                    item_shape: IxDyn(&shape[dim..]),
                })
            }
            &MoveAxis([tensor_id, src_axis_id, dest_axis_id]) => {
                let mut new_shape = Self::get_shape(tensor_id, egraph).clone();
                let src_axis = Self::get_usize(src_axis_id, egraph);
                let dest_axis = Self::get_usize(dest_axis_id, egraph);

                assert!(src_axis < new_shape.as_array_view().len());
                assert!(dest_axis < new_shape.as_array_view().len());

                let tmp = new_shape[dest_axis];
                new_shape[dest_axis] = new_shape[src_axis];
                new_shape[src_axis] = tmp;

                MyAnalysisData::Legacy(MyAnalysisDataLegacyData {
                    shape: Some(new_shape),
                    usize_value: None,
                })
            }
            &CartesianProduct([t0_id, t1_id]) => {
                let initial_shape_left: &IxDyn = Self::get_shape(t0_id, egraph);
                assert!(initial_shape_left.as_array_view().len() >= 1);
                assert!(initial_shape_left.as_array_view().len() <= 2);
                let initial_shape_right: &IxDyn = Self::get_shape(t1_id, egraph);
                assert!(initial_shape_left.as_array_view().len() >= 1);
                assert!(initial_shape_left.as_array_view().len() <= 2);
                assert_eq!(
                    initial_shape_left[initial_shape_left.as_array_view().len() - 1],
                    initial_shape_right[initial_shape_right.as_array_view().len() - 1],
                );

                // New shape is [a1, ..., an, b1, ..., bn, 2, c].
                let mut new_shape: Vec<usize> = initial_shape_left
                    .as_array_view()
                    .iter()
                    .take(initial_shape_left.as_array_view().len() - 1)
                    .copied()
                    .collect();
                new_shape.extend(
                    initial_shape_right
                        .as_array_view()
                        .iter()
                        .take(initial_shape_right.as_array_view().len() - 1),
                );
                new_shape.push(2);
                new_shape.push(initial_shape_left[initial_shape_left.as_array_view().len() - 1]);
                let new_shape: ndarray::IxDyn = ndarray::IxDyn(&new_shape[..]);
                assert_eq!(
                    new_shape.as_array_view().len(),
                    initial_shape_left.as_array_view().len() - 1
                        + initial_shape_right.as_array_view().len()
                        - 1
                        + 1
                        + 1
                );
                MyAnalysisData::Legacy(MyAnalysisDataLegacyData {
                    shape: Some(new_shape),
                    usize_value: None,
                })
            }
            &MapDotProduct(tensor_id) => {
                let shape: &IxDyn = Self::get_shape(tensor_id, egraph);

                assert!(shape.as_array_view().len() >= 3);
                assert_eq!(shape[shape.as_array_view().len() - 2], 2);

                let new_shape: ndarray::IxDyn = ndarray::IxDyn(
                    &shape
                        .as_array_view()
                        .iter()
                        .take(shape.as_array_view().len() - 2)
                        .copied()
                        .collect::<Vec<usize>>()[..],
                );

                MyAnalysisData::Legacy(MyAnalysisDataLegacyData {
                    shape: Some(new_shape),
                    usize_value: None,
                })
            }
            &BsgSystolicArray([rows_id, cols_id, t0_id, t1_id]) => {
                // Check that the rows and cols are usizes.
                let _unused = Self::get_usize(rows_id, egraph);
                let _unused = Self::get_usize(cols_id, egraph);

                let left_shape = Self::get_shape(t0_id, egraph);
                let right_shape = Self::get_shape(t1_id, egraph);
                let left_shape_len: usize = left_shape.as_array_view().len();
                let right_shape_len: usize = right_shape.as_array_view().len();

                // TODO(@gussmith23) check that the rows/cols params sizes are correct
                // given the input tensor shapes.

                // Assumptions I'm making right now.
                assert!(left_shape_len == 1 || left_shape_len == 2);
                assert_eq!(right_shape_len, 2);

                let new_shape: Vec<ndarray::Ix> = left_shape
                    .as_array_view()
                    .iter()
                    .cloned()
                    .take(left_shape.as_array_view().len() - 1)
                    .chain(right_shape.as_array_view().iter().cloned().skip(1))
                    .collect();

                MyAnalysisData::Legacy(MyAnalysisDataLegacyData {
                    shape: Some(ndarray::IxDyn(&new_shape)),
                    usize_value: None,
                })
            }
            &SystolicArray([rows_id, cols_id, a0_id, a1_id])
            | &SystolicArrayWithBlocking([rows_id, cols_id, a0_id, a1_id]) => {
                let rows = Self::get_usize(rows_id, egraph);
                let cols = Self::get_usize(cols_id, egraph);

                let (a0, a1) = match (&egraph[a0_id].data, &egraph[a1_id].data) {
                    (MyAnalysisData::AccessPattern(a0), MyAnalysisData::AccessPattern(a1)) => {
                        (a0, a1)
                    }
                    _ => panic!("Expected access patterns as third and fourth arguments"),
                };

                assert_eq!(a1.shape, IxDyn(&[]));
                assert!(a0.shape.ndim() == 0 || a0.shape.ndim() == 1);

                match &enode {
                    &SystolicArray(_) => {
                        assert_eq!(a1.item_shape, IxDyn(&[rows, cols]));
                        assert_eq!(a0.item_shape, IxDyn(&[rows]));
                    }
                    &SystolicArrayWithBlocking(_) => {
                        // Scott: The input vector size should be a multiple of
                        // the systolic array's height and the output vector
                        // size should be a multiple of the systolic array's
                        // width.
                        assert_eq!(a0.item_shape.ndim(), 1);
                        assert!(a0.item_shape.slice()[0] % rows == 0);
                        assert_eq!(a1.item_shape.ndim(), 2);
                        assert_eq!(a0.item_shape.slice()[0], a1.item_shape.slice()[0]);
                        assert!(a1.item_shape.slice()[1] % cols == 0);
                    }
                    _ => unreachable!(),
                }

                MyAnalysisData::AccessPattern(AccessPatternData {
                    // TODO(@gussmith23) Implement zero regions
                    // It's harmless (I think) if `zero_regions` defaults to
                    // empty, but for it to be useful, we need to implement it
                    // for each operator.
                    zero_regions: {
                        if !a0.zero_regions.is_empty() {
                            warn!(
                                "Throwing away zero region analysis data on line {}",
                                std::line!()
                            );
                        }
                        if !a1.zero_regions.is_empty() {
                            warn!(
                                "Throwing away zero region analysis data on line {}",
                                std::line!()
                            );
                        }
                        HashMap::default()
                    },
                    shape: IxDyn(
                        a0.shape
                            .as_array_view()
                            .iter()
                            .chain(std::iter::once(&a1.item_shape.slice()[1]))
                            .cloned()
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ),
                    item_shape: IxDyn(&[]),
                })
            }
            &Slice([tensor_id, axis_id, low_id, high_id]) => {
                let mut new_shape: IxDyn = Self::get_shape(tensor_id, egraph).clone();

                let axis: usize = Self::get_usize(axis_id, egraph);
                let low: usize = Self::get_usize(low_id, egraph);
                let high: usize = Self::get_usize(high_id, egraph);

                assert!(new_shape.as_array_view().len() > axis);
                assert!(low < new_shape[axis]);
                assert!(high <= new_shape[axis]);

                new_shape[axis] = high - low;

                MyAnalysisData::Legacy(MyAnalysisDataLegacyData {
                    shape: Some(new_shape),
                    usize_value: None,
                })
            }
            &Concatenate([t0_id, t1_id, axis_id]) => {
                let axis = Self::get_usize(axis_id, egraph);
                let mut new_shape = Self::get_shape(t0_id, egraph).clone();
                let t1_shape = Self::get_shape(t1_id, egraph).clone();
                assert_eq!(
                    new_shape.as_array_view().len(),
                    t1_shape.as_array_view().len()
                );
                assert!(axis < t1_shape.as_array_view().len());
                new_shape[axis] += t1_shape[axis];

                MyAnalysisData::Legacy(MyAnalysisDataLegacyData {
                    shape: Some(new_shape),
                    usize_value: None,
                })
            }
            &ElementwiseAdd([t0_id, t1_id]) => {
                assert_eq!(
                    Self::get_shape(t0_id, egraph),
                    Self::get_shape(t1_id, egraph)
                );

                MyAnalysisData::Legacy(MyAnalysisDataLegacyData {
                    shape: Some(Self::get_shape(t0_id, egraph).clone()),
                    usize_value: None,
                })
            }
            Usize(u) => MyAnalysisData::Legacy(MyAnalysisDataLegacyData {
                shape: None,
                usize_value: Some(*u),
            }),
            Symbol(name) => {
                //println!("Symbol");
                MyAnalysisData::Legacy(MyAnalysisDataLegacyData {
                    shape: Some(ndarray::IxDyn(
                        &(match &name[..] {
                            "in" => vec![1, 784],
                            "w1" => vec![784, 512],
                            "w2" => vec![512, 512],
                            "w3" => vec![512, 10],
                            // TODO(@gussmith23) have to figure out a way around this.
                            // Max seems to think the tensors should just go
                            // into the egraph. I was hoping to have some kind
                            // of environment that we could wrap the egraph in
                            // (would have to be accessible from here), but Max
                            // doesn't have that nor does he plan to implement
                            // it.
                            //
                            // Update, Max is implementing something that will
                            // allow for this.
                            "single-matrix-multiply-input-a" => vec![32, 32],
                            "single-matrix-multiply-input-b" => vec![32, 32],
                            "v-32" => vec![32],
                            "t-32-32" => vec![32, 32],
                            "t-32-64" => vec![32, 64],
                            "t-64-128" => vec![64, 128],
                            "t-128-16" => vec![128, 16],
                            // A 3-channel "image" in CHW format.
                            "t-3-32-32" => vec![3, 32, 32],
                            // An OIHW set of convolution filters.
                            "t-8-3-3-3" => vec![8, 3, 3, 3],
                            "t-1024-2-256" => vec![1024, 2, 256],
                            "t-1-2-3-4" => vec![1, 2, 3, 4],
                            _ => egraph
                                .analysis
                                .name_to_shape
                                .get(name)
                                .unwrap_or_else(|| panic!("No shape defined for {}", name))
                                .clone(),
                        })[..],
                    )),
                    usize_value: None,
                })
            }
            PadType(t) => MyAnalysisData::PadType(*t),
            &AccessWindows([access_id, filters_shape_id, stride_shape_id]) => {
                let access = match &egraph[access_id].data {
                    MyAnalysisData::AccessPattern(a) => a,
                    _ => {
                        panic!("Expected an access pattern as the first argument to access-windows")
                    }
                };
                let filters_shape = MyAnalysis::get_shape_of_value(filters_shape_id, egraph);
                let stride_shape = MyAnalysis::get_shape_of_value(stride_shape_id, egraph);

                // TODO(@gussmith23) Generalize AccessWindows to other accesses
                // Right now we expect item shape to be a scalar.
                assert_eq!(access.item_shape.ndim(), 0);

                MyAnalysisData::AccessPattern(AccessPatternData {
                    // TODO(@gussmith23) Implement zero regions
                    // It's harmless (I think) if `zero_regions` defaults to
                    // empty, but for it to be useful, we need to implement it
                    // for each operator.
                    zero_regions: {
                        if !access.zero_regions.is_empty() {
                            warn!(
                                "Throwing away zero region analysis data on line {}",
                                std::line!()
                            );
                        }
                        HashMap::default()
                    },
                    shape: IxDyn(
                        access_windows_resulting_shape(
                            &access.shape,
                            &filters_shape,
                            &stride_shape,
                        )
                        .as_slice(),
                    ),
                    item_shape: filters_shape.clone(),
                })
            }

            &ShapeOf([tensor_id]) => MyAnalysisData::Shape(ShapeData {
                shape: MyAnalysis::get_shape(tensor_id, egraph).clone(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        "
         (map-dot-product
          (cartesian-product
           single-matrix-multiply-input-a
           (move-axis single-matrix-multiply-input-b 1 0)
          )
         )
         "
        .parse::<egg::RecExpr<Language>>()
        .unwrap();
    }

    #[test]
    fn test_cartesian_product_shape() {
        let program = "(cartesian-product
          v-32
          (move-axis t-32-32 1 0)
         )
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        assert_eq!(MyAnalysis::get_shape(id, &egraph), &IxDyn(&[32, 2, 32]));

        let program = "(cartesian-product
          (move-axis t-32-32 1 0)
          v-32
         )
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        assert_eq!(MyAnalysis::get_shape(id, &egraph), &IxDyn(&[32, 2, 32]));
    }

    #[test]
    fn access_windows() {
        // TODO(@gussmith23) Could probably clean this up with a for loop
        // Would make it easier to add more tests.

        let program = "
         (access-windows (access (access-tensor t-3-32-32) 3) (slice-shape (shape-of t-8-3-3-3) 1) (shape 1 1 1))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[1, 30, 30]));
                assert_eq!(a.item_shape, IxDyn(&[3, 3, 3]));
            }
            _ => panic!(),
        }

        let program = "
         (access-windows (access (access-tensor t-3-32-32) 3) (slice-shape (shape-of t-8-3-3-3) 1) (shape 1 2 1))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[1, 15, 30]));
                assert_eq!(a.item_shape, IxDyn(&[3, 3, 3]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn shape_of() {
        let program = "
         (shape-of t-3-32-32)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        assert_eq!(
            MyAnalysis::get_shape_of_value(id, &egraph),
            &IxDyn(&[3, 32, 32])
        );
    }

    #[test]
    fn access() {
        let program = "
         (access (access-tensor t-3-32-32) 0)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[3, 32, 32]));
            }
            _ => panic!(),
        }

        let program = "
         (access (access-tensor t-3-32-32) 2)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3, 32]));
                assert_eq!(a.item_shape, IxDyn(&[32]));
            }
            _ => panic!(),
        }

        let program = "
         (access (access-tensor t-3-32-32) 3)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3, 32, 32]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn reaccess() {
        let program = "
         (access (access (access-tensor t-3-32-32) 3) 0)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[3, 32, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    #[should_panic]
    fn access_invalid() {
        let program = "
         (access (access-tensor t-3-32-32) 4)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        egraph.add_expr(&program);
    }

    #[test]
    fn shape_insert_axis_0() {
        let program = "
         (shape-insert-axis (shape 1 2 3) 2)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        assert_eq!(
            MyAnalysis::get_shape_of_value(id, &egraph),
            &IxDyn(&[1, 2, 1, 3])
        );
    }

    #[test]
    fn shape_insert_axis_1() {
        let program = "
         (shape-insert-axis (shape 1 2 3) 3)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        assert_eq!(
            MyAnalysis::get_shape_of_value(id, &egraph),
            &IxDyn(&[1, 2, 3, 1])
        );
    }

    #[test]
    fn shape_remove_axis_0() {
        let program = "
         (shape-remove-axis (shape 1 2 3) 0)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        assert_eq!(MyAnalysis::get_shape_of_value(id, &egraph), &IxDyn(&[2, 3]));
    }

    #[test]
    fn shape_remove_axis_1() {
        let program = "
         (shape-remove-axis (shape 1 2 3) 1)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        assert_eq!(MyAnalysis::get_shape_of_value(id, &egraph), &IxDyn(&[1, 3]));
    }

    #[test]
    fn shape_remove_axis_2() {
        let program = "
         (shape-remove-axis (shape 1 2 3) 2)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        assert_eq!(MyAnalysis::get_shape_of_value(id, &egraph), &IxDyn(&[1, 2]));
    }

    #[test]
    #[should_panic]
    fn shape_remove_axis_panic() {
        let program = "
         (shape-remove-axis (shape 1 2 3) 3)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        egraph.add_expr(&program);
    }

    #[test]
    #[should_panic]
    fn shape_insert_axis_panic() {
        let program = "
         (shape-insert-axis (shape 1 2 3) 4)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        egraph.add_expr(&program);
    }

    #[test]
    fn slice_shape() {
        let program = "
         (slice-shape (shape-of t-3-32-32) 2)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        assert_eq!(MyAnalysis::get_shape_of_value(id, &egraph), &IxDyn(&[32]));

        let program = "
         (slice-shape (shape-of t-3-32-32) 0)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        assert_eq!(
            MyAnalysis::get_shape_of_value(id, &egraph),
            &IxDyn(&[3, 32, 32])
        );
    }

    #[test]
    #[should_panic]
    fn slice_shape_invalid_slice() {
        let program = "
         (slice-shape (shape-of t-3-32-32) 10)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        assert_eq!(MyAnalysis::get_shape_of_value(id, &egraph), &IxDyn(&[]));
    }

    #[test]
    fn access_cartesian_product() {
        let program = "
         (access-cartesian-product
          (access (access-tensor v-32) 0)
          (access (access-tensor t-32-32) 1)
         )
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32]));
                assert_eq!(a.item_shape, IxDyn(&[2, 32]));
            }
            _ => panic!(),
        }

        let program = "
         (access-cartesian-product
          (access (access-tensor t-32-32) 1)
          (access (access-tensor v-32) 0)
         )
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32]));
                assert_eq!(a.item_shape, IxDyn(&[2, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    // TODO(@gussmith23) More tests of cart prod w/ padding
    fn access_cartesian_product_zero_padding() {
        let program = "
         (access-cartesian-product
          (access-pad
           (access (access-tensor v-32) 0)
           zero-padding 0 2 3
          )
          (access-pad
           (access (access-tensor t-32-32) 1)
           zero-padding 1 2 3
          )
         )
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32]));
                assert_eq!(a.item_shape, IxDyn(&[2, 37]));
                assert_eq!(a.zero_regions.len(), 1);
                assert_eq!(a.zero_regions[&2].len(), 37);
                assert!(a.zero_regions[&2].covered((0, 2)));
                assert!(!a.zero_regions[&2].covered((2, 34)));
                assert!(a.zero_regions[&2].covered((34, 37)));
                assert_eq!(
                    a.zero_regions[&2],
                    std::iter::repeat(true)
                        .take(2)
                        .chain(std::iter::repeat(false).take(32))
                        .chain(std::iter::repeat(true).take(3))
                        .collect::<Vec<_>>()
                )
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_dot_product() {
        let program = "
         (compute dot-product (access (access-tensor t-3-32-32) 0))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }

        let program = "
         (compute dot-product (access (access-tensor t-3-32-32) 1))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }

        let program = "
         (compute dot-product (access (access-tensor t-3-32-32) 2))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3, 32]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    // This may not panic in the future, if we allow dot products over empty
    // tuples.
    #[should_panic]
    #[test]
    fn compute_dot_product_panic() {
        let program = "
         (compute dot-product (access (access-tensor t-3-32-32) 3))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3, 32]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn conv2d() {
        // The following TVM/Python code will compute the correct sizes of
        // cov2ds.
        //
        // import tvm
        // from tvm import relay
        //
        // mod = relay.module.Module.from_expr(
        //     relay.nn.conv2d(relay.var('x', shape=[1, 3, 32, 32]),
        //                     relay.var('weight', shape=[8, 3, 3, 3])))
        //
        // print(mod)

        let program = "
         (compute dot-product
          (access-cartesian-product
           (access (access-tensor t-8-3-3-3) 1)
           (access-squeeze
            (access-windows
             (access (access-tensor t-3-32-32) 3)
             (slice-shape (shape-of t-8-3-3-3) 1)
             (shape 1 1 1)
            )
            0
           )
          )
         )
        "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);

        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[8, 30, 30]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }

        let program = "
         (compute dot-product
          (access-cartesian-product
           (access (access-tensor t-8-3-3-3) 1)
           (access-squeeze
            (access-windows
             (access (access-tensor t-3-32-32) 3)
             (slice-shape (shape-of t-8-3-3-3) 1)
             (shape 1 1 2)
            )
            0
           )
          )
         )
        "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);

        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[8, 30, 15]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn flatten_reshape() {
        let program = "
         (access-reshape
          (access-flatten (access (access-tensor t-3-32-32) 2))
          (access-shape (shape 32 3) (shape 16 2))
         )
        "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);

        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32, 3]));
                assert_eq!(a.item_shape, IxDyn(&[16, 2]));
            }
            _ => panic!(),
        }
    }

    #[should_panic]
    #[test]
    fn flatten_reshape_panic() {
        let program = "
         (access-reshape
          (access-flatten (access (access-tensor t-3-32-32) 2))
          (access-shape (shape 1) (shape 16 2))
         )
        "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);

        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32, 3]));
                assert_eq!(a.item_shape, IxDyn(&[16, 2]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_slice_0() {
        let program = "(access-slice (access (access-tensor t-3-32-32) 1) 0 0 1)"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[1]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_slice_1() {
        let program = "(access-slice (access (access-tensor t-3-32-32) 1) 1 16 32)"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3]));
                assert_eq!(a.item_shape, IxDyn(&[16, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_slice_2() {
        let program = "(access-slice (access (access-tensor t-3-32-32) 2) 2 16 32)"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3, 32]));
                assert_eq!(a.item_shape, IxDyn(&[16]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_slice_zero_pad_0() {
        test_logger::ensure_env_logger_initialized();

        let program = "(access-slice (access-pad (access (access-tensor t-3-32-32) 1) zero-padding 0 2 3) 0 0 3)"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
                assert_eq!(a.zero_regions.len(), 1);
                assert_eq!(a.zero_regions[&0].len(), 3);
                assert_eq!(a.zero_regions[&0], vec![true, true, false]);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_slice_zero_pad_1() {
        test_logger::ensure_env_logger_initialized();

        let program = "
(access-slice
 (access-pad
  (access (access-tensor t-3-32-32) 1)
  zero-padding 0 2 3
 )
 0 1 7
)"
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[6]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
                assert_eq!(a.zero_regions.len(), 1);
                assert_eq!(a.zero_regions[&0].len(), 6);
                assert_eq!(
                    a.zero_regions[&0],
                    vec![true, false, false, false, true, true]
                );
            }
            _ => panic!(),
        }
    }

    #[test]
    #[should_panic]
    fn access_slice_panic() {
        let program = "(access-slice (access (access-tensor t-3-32-32) 1) 3 16 32)"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3]));
                assert_eq!(a.item_shape, IxDyn(&[16, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_concatenate_0() {
        let program = "(access-concatenate (access (access-tensor t-3-32-32) 1) (access (access-tensor t-3-32-32) 1) 0)"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[6]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_concatenate_1() {
        let program = "(access-concatenate (access (access-tensor t-3-32-32) 1) (access (access-tensor t-3-32-32) 1) 2)"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3]));
                assert_eq!(a.item_shape, IxDyn(&[32, 64]));
            }
            _ => panic!(),
        }
    }

    #[should_panic]
    #[test]
    fn access_concatenate_panic_0() {
        let program = "(access-concatenate (access (access-tensor t-3-32-32) 1) (access (access-tensor t-3-32-32) 1) 3)"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3]));
                assert_eq!(a.item_shape, IxDyn(&[32, 64]));
            }
            _ => panic!(),
        }
    }

    #[should_panic]
    #[test]
    fn access_concatenate_panic_1() {
        let program = "(access-concatenate (access (access-tensor t-3-32-32) 1) (access (access-tensor t-8-3-3-3) 1) 2)"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3]));
                assert_eq!(a.item_shape, IxDyn(&[32, 64]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_transpose_0() {
        let program = "(access-transpose (access (access-tensor t-3-32-32) 1) (list 1 2 0))"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32]));
                assert_eq!(a.item_shape, IxDyn(&[32, 3]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_transpose_4() {
        let program = "(access-transpose (access (access-tensor t-3-32-32) 1) (list 1 0 2))"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32]));
                assert_eq!(a.item_shape, IxDyn(&[3, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_transpose_5() {
        let program = "(access-transpose (access (access-tensor t-3-32-32) 1) (list 0 1 2))"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[should_panic]
    #[test]
    fn access_transpose_panic_2() {
        let program = "(access-transpose (access (access-tensor t-3-32-32) 1) (list 0 1 3))"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[should_panic]
    #[test]
    fn access_move_axis_panic_1() {
        let program = "(access-move-axis (access (access-tensor t-3-32-32) 1) 1 3)"
            .parse()
            .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_reduce_sum_0() {
        let program = "
         (compute reduce-sum (access (access-tensor t-3-32-32) 0))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_reduce_sum_1() {
        let program = "
         (compute reduce-sum (access (access-tensor t-3-32-32) 2))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3, 32]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_pair() {
        let program = "
         (access-pair (access (access-tensor t-32-32) 1) (access (access-tensor t-32-32) 1))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32]));
                assert_eq!(a.item_shape, IxDyn(&[2, 32]));
            }
            _ => panic!(),
        }
    }

    #[should_panic]
    #[test]
    fn access_pair_panic() {
        let program = "
         (access-pair (access (access-tensor t-32-32) 0) (access (access-tensor t-32-32) 1))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32]));
                assert_eq!(a.item_shape, IxDyn(&[2, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_shift_right_0() {
        let program = "
         (access-shift-right (access (access-tensor t-32-32) 2))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32]));
                assert_eq!(a.item_shape, IxDyn(&[32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_shift_right_1() {
        let program = "
         (access-shift-right (access (access-tensor t-32-32) 1))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_shift_right_2() {
        let program = "
         (access-shift-right (access (access-tensor t-32-32) 0))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_relu() {
        let program = "
         (compute relu (access (access-tensor t-32-32) 0))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_elementwise_add_0() {
        let program = "
         (compute elementwise-add (access (access-tensor t-3-32-32) 0))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_elementwise_add_1() {
        let program = "
         (compute elementwise-add (access (access-tensor t-3-32-32) 1))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3]));
                assert_eq!(a.item_shape, IxDyn(&[32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_elementwise_add_2() {
        let program = "
         (compute elementwise-add (access (access-tensor t-3-32-32) 2))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3, 32]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[should_panic]
    #[test]
    fn compute_elementwise_add_panic() {
        let program = "
         (compute elementwise-add (access (access-tensor t-3-32-32) 3))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        egraph.add_expr(&program);
    }

    #[test]
    fn compute_elementwise_mul_0() {
        let program = "
         (compute elementwise-mul (access (access-tensor t-3-32-32) 0))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_elementwise_mul_1() {
        let program = "
         (compute elementwise-mul (access (access-tensor t-3-32-32) 1))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3]));
                assert_eq!(a.item_shape, IxDyn(&[32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_elementwise_mul_2() {
        let program = "
         (compute elementwise-mul (access (access-tensor t-3-32-32) 2))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3, 32]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[should_panic]
    #[test]
    fn compute_elementwise_mul_panic() {
        let program = "
         (compute elementwise-mul (access (access-tensor t-3-32-32) 3))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        egraph.add_expr(&program);
    }

    #[test]
    fn zero_padding() {
        let program = "zero-padding".parse().unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::PadType(PadType::ZeroPadding) => (),
            _ => panic!(),
        };
    }

    #[test]
    fn access_pad_zero_padding_0() {
        let program = "
         (access-pad (access (access-tensor t-32-32) 1) zero-padding 0 1 2)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[35]));
                assert_eq!(a.item_shape, IxDyn(&[32]));
                assert_eq!(a.zero_regions.len(), 1);
                assert_eq!(a.zero_regions[&0].len(), 35);
                assert!(a.zero_regions[&0].covered((0, 1)));
                assert!(!a.zero_regions[&0].covered((1, 33)));
                assert!(a.zero_regions[&0].covered((33, 35)));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_pad_zero_padding_1() {
        let program = "
         (access-pad (access (access-tensor t-32-32) 1) zero-padding 1 0 2)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32]));
                assert_eq!(a.item_shape, IxDyn(&[34]));
                assert_eq!(a.zero_regions.len(), 1);
                assert_eq!(a.zero_regions[&1].len(), 34);
                assert!(a.zero_regions[&1].covered((32, 34)));
                assert!(!a.zero_regions[&1].covered((0, 32)));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_pad_zero_padding_2() {
        let program = "
(access-pad
 (access-pad (access (access-tensor t-32-32) 1) zero-padding 1 0 2)
 zero-padding 0 1 3
)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[36]));
                assert_eq!(a.item_shape, IxDyn(&[34]));
                assert_eq!(a.zero_regions.len(), 2);
                assert_eq!(a.zero_regions[&1].len(), 34);
                assert_eq!(a.zero_regions[&0].len(), 36);
                assert!(a.zero_regions[&1].covered((32, 34)));
                assert!(!a.zero_regions[&1].covered((0, 32)));
                assert!(a.zero_regions[&0].covered((0, 1)));
                assert!(a.zero_regions[&0].covered((33, 36)));
                assert!(!a.zero_regions[&0].covered((1, 33)));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_pad_zero_padding_3() {
        let program = "
(access-pad
 (access-pad (access (access-tensor t-32-32) 1) zero-padding 0 0 2)
 zero-padding 0 1 3
)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[38]));
                assert_eq!(a.item_shape, IxDyn(&[32]));
                assert_eq!(a.zero_regions.len(), 1);
                assert_eq!(a.zero_regions[&0].len(), 38);
                assert!(a.zero_regions[&0].covered((0, 1)));
                assert!(a.zero_regions[&0].covered((35, 38)));
                assert!(!a.zero_regions[&0].covered((1, 35)));
                // This one is key: this makes sure that the first pad's zero
                // region was shifted appropriately by the second pad (was (32,
                // 34), but should get shifted by 1)
                assert!(a.zero_regions[&0].covered((33, 35)));
                assert!(!a.zero_regions[&0].covered((0, 33)));
            }
            _ => panic!(),
        }
    }

    #[test]
    #[should_panic]
    fn access_pad_zero_padding_panic() {
        let program = "
         (access-pad (access (access-tensor t-32-32) 1) zero-padding 2 3 2)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        egraph.add_expr(&program);
    }

    #[test]
    fn compute_reduce_max_0() {
        let program = "
         (compute reduce-max (access (access-tensor t-3-32-32) 0))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_reduce_max_1() {
        let program = "
         (compute reduce-max (access (access-tensor t-3-32-32) 2))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3, 32]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_squeeze() {
        let program = "
         (access-squeeze (access (access-tensor t-1-2-3-4) 1) 0)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[2, 3, 4]));
            }
            _ => panic!(),
        }
    }

    #[test]
    #[should_panic]
    fn access_squeeze_panic() {
        let program = "
         (access-squeeze (access (access-tensor t-1-2-3-4) 1) 2)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        egraph.add_expr(&program);
    }

    #[test]
    fn access_insert_axis() {
        let program = "
         (access-insert-axis (access (access-tensor t-1-2-3-4) 1) 0)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[1, 1]));
                assert_eq!(a.item_shape, IxDyn(&[2, 3, 4]));
            }
            _ => panic!(),
        }
    }

    #[test]
    #[should_panic]
    // TODO(@gussmith) More access-insert-axis tests
    fn access_insert_axis_panic() {
        let program = "
         (access-squeeze (access (access-tensor t-1-2-3-4) 1) 5)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        egraph.add_expr(&program);
    }

    #[test]
    // TODO(@gussmith) More access-broadcast tests
    fn access_broadcast() {
        let program = "
         (access-broadcast (access (access-tensor t-1-2-3-4) 1) (access-shape (shape 2 2 3 4) (shape)))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis {
            name_to_shape: HashMap::default(),
        });
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[2]));
                assert_eq!(a.item_shape, IxDyn(&[2, 3, 4]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn systolic_array() {
        let mut map = HashMap::default();
        map.insert("a".to_string(), vec![32, 64]);
        let program = "
         (systolic-array 64 32
          (access (access-tensor a) 1)
          (access (access-transpose (access-tensor a) (list 1 0)) 0)
         )
         "
        .parse()
        .unwrap();
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32, 32]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    #[should_panic]
    fn systolic_array_panic_0() {
        let mut map = HashMap::default();
        map.insert("a".to_string(), vec![32, 64]);
        // Because the second argument is not the right shape.
        let program = "
         (systolic-array 64 32
          (access (access-tensor a) 1)
          (access (access-tensor a) 0)
         )
         "
        .parse()
        .unwrap();
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        egraph.add_expr(&program);
    }

    #[test]
    #[should_panic]
    fn systolic_array_panic_1() {
        let mut map = HashMap::default();
        map.insert("a".to_string(), vec![32, 64]);
        // Because the second argument is not accessed at the right axis.
        let program = "
         (systolic-array 64 32
          (access (access-tensor a) 1)
          (access (move-axis (access-tensor a) 0 1) 1)
         )
         "
        .parse()
        .unwrap();
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        egraph.add_expr(&program);
    }

    #[test]
    fn list() {
        let program = "
         (list 1 2 3 4)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::List(l) => assert_eq!(l, &vec![1, 2, 3, 4]),
            _ => panic!(),
        }
    }

    #[test]
    fn access_transpose() {
        let program = "
         (access-transpose (access (access-tensor a) 1) (list 2 0 1))
         "
        .parse()
        .unwrap();
        let mut map = HashMap::default();
        map.insert("a".to_string(), vec![4, 5, 6]);
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[6]));
                assert_eq!(a.item_shape, IxDyn(&[4, 5]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_transpose_1() {
        let program = "
             (access-transpose
              (access-transpose
               (access (access-tensor t) 1)
               (list 1 3 2 0)
              )
              (list 3 2 1 0)
             )"
        .parse()
        .unwrap();
        let mut map = HashMap::new();
        map.insert("t".to_string(), vec![1, 2, 3, 4]);
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[1]));
                assert_eq!(a.item_shape, IxDyn(&[3, 4, 2]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_transpose_2() {
        let program = "
              (access-transpose
               (access (access-tensor t) 1)
               (list 1 3 2 0)
              )
             "
        .parse()
        .unwrap();
        let mut map = HashMap::new();
        map.insert("t".to_string(), vec![1, 2, 3, 4]);
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[2]));
                assert_eq!(a.item_shape, IxDyn(&[4, 3, 1]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_transpose_3() {
        let program = "
              (access-transpose
               (access-pad (access (access-tensor t) 1) zero-padding 1 5 0)
               (list 1 3 2 0)
              )
             "
        .parse()
        .unwrap();
        let mut map = HashMap::new();
        map.insert("t".to_string(), vec![1, 2, 3, 4]);
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[7]));
                assert_eq!(a.item_shape, IxDyn(&[4, 3, 1]));
                assert_eq!(a.zero_regions.len(), 1);
                assert_eq!(a.zero_regions[&0].len(), 7);
                assert!(a.zero_regions[&0].covered((0, 5)));
                assert!(!a.zero_regions[&0].covered((5, 7)));
            }
            _ => panic!(),
        }
    }

    #[test]
    #[should_panic]
    fn access_transpose_panic_0() {
        let program = "
         (access-transpose (access (access-tensor a) 1) (list 0 1))
         "
        .parse()
        .unwrap();
        let mut map = HashMap::default();
        map.insert("a".to_string(), vec![4, 5, 6]);
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        egraph.add_expr(&program);
    }

    #[test]
    #[should_panic]
    fn access_transpose_panic_1() {
        let program = "
         (access-transpose (access (access-tensor a) 1) (list 2 1 1))
         "
        .parse()
        .unwrap();
        let mut map = HashMap::default();
        map.insert("a".to_string(), vec![4, 6]);
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        egraph.add_expr(&program);
    }

    #[test]
    #[should_panic]
    fn compute_softmax_0() {
        let program = "
         (compute softmax (access (access-tensor t-3-32-32) 3))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        egraph.add_expr(&program);
    }

    #[test]
    #[should_panic]
    fn compute_softmax_1() {
        let program = "
         (compute softmax (access (access-tensor t-3-32-32) 0))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        egraph.add_expr(&program);
    }

    #[test]
    fn compute_softmax_2() {
        let program = "
         (compute softmax (access (access-tensor t-3-32-32) 2))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3, 32]));
                assert_eq!(a.item_shape, IxDyn(&[32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_reduce_mean_0() {
        let program = "
         (compute reduce-mean (access (access-tensor t-3-32-32) 0))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_reduce_mean_1() {
        let program = "
         (compute reduce-mean (access (access-tensor t-3-32-32) 2))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[3, 32]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_pad_min_padding() {
        let program = "
         (access-pad (access (access-tensor t-32-32) 1) min-padding 0 1 2)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[35]));
                assert_eq!(a.item_shape, IxDyn(&[32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_elementwise_div() {
        let program = "
         (compute elementwise-div (access (access-tensor t-3-32-32) 0))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn literal_0() {
        let program = "
         (literal 0.1234)
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::Literal(t) => {
                assert_eq!(*t, ndarray::arr0(0.1234).into_dyn());
            }
            _ => panic!(),
        }
    }

    #[test]
    fn access_literal() {
        let program = "
         (access-literal (literal 0.1234))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_sqrt() {
        let program = "
         (compute sqrt (access (access-tensor t-32-32) 0))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn compute_negative() {
        let program = "
         (compute negative (access (access-tensor t-32-32) 0))
         "
        .parse()
        .unwrap();
        let mut egraph = egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis::default());
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[]));
                assert_eq!(a.item_shape, IxDyn(&[32, 32]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn systolic_array_with_blocking_0() {
        let mut map = HashMap::default();
        map.insert("a".to_string(), vec![32, 64]);
        map.insert("b".to_string(), vec![64, 32]);
        let program = "
         (systolic-array-with-blocking 64 32
          (access (access-tensor a) 1)
          (access (access-tensor b) 0)
         )
         "
        .parse()
        .unwrap();
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32, 32]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn systolic_array_with_blocking_2() {
        let mut map = HashMap::default();
        map.insert("a".to_string(), vec![32, 64]);
        map.insert("b".to_string(), vec![64, 32]);
        let program = "
         (systolic-array-with-blocking 32 32
          (access (access-tensor a) 1)
          (access (access-tensor b) 0)
         )
         "
        .parse()
        .unwrap();
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32, 32]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn systolic_array_with_blocking_3() {
        let mut map = HashMap::default();
        map.insert("a".to_string(), vec![32, 64]);
        map.insert("b".to_string(), vec![64, 32]);
        let program = "
         (systolic-array-with-blocking 32 2
          (access (access-tensor a) 1)
          (access (access-tensor b) 0)
         )
         "
        .parse()
        .unwrap();
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        let id = egraph.add_expr(&program);
        match &egraph[id].data {
            MyAnalysisData::AccessPattern(a) => {
                assert_eq!(a.shape, IxDyn(&[32, 32]));
                assert_eq!(a.item_shape, IxDyn(&[]));
            }
            _ => panic!(),
        }
    }

    #[test]
    #[should_panic(expected = "assertion failed: a1.item_shape.slice()[1] % cols == 0")]
    fn systolic_array_with_blocking_panic() {
        let mut map = HashMap::default();
        map.insert("a".to_string(), vec![32, 64]);
        map.insert("b".to_string(), vec![64, 32]);
        let program = "
         (systolic-array-with-blocking 32 3
          (access (access-tensor a) 1)
          (access (access-tensor b) 0)
         )
         "
        .parse()
        .unwrap();
        let mut egraph =
            egg::EGraph::<Language, MyAnalysis>::new(MyAnalysis { name_to_shape: map });
        let _id = egraph.add_expr(&program);
    }
}
