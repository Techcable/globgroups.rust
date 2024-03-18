use crate::{GlobExpr, GlobExprKind, Text};
use itertools::{Itertools, Product};
/// The underlying expansion
///
/// The current implementation is not very performant
/// and performs many intermediate allocations.
use std::iter::FusedIterator;

/// Expands a [`GlobExpr`]
///
/// ## Example
///
pub struct ExpandGlobIter<'a> {
    state: ExpandState<'a>,
}
impl<'a> ExpandGlobIter<'a> {
    pub(crate) fn new(glob: &'a GlobExpr) -> Self {
        ExpandGlobIter {
            state: ExpandState::new(glob),
        }
    }
}

impl FusedIterator for ExpandGlobIter<'_> {}
impl Iterator for ExpandGlobIter<'_> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.state.next().map(|parts| {
            let total_len = parts.iter().map(|part| part.len()).sum();
            let mut buffer = String::with_capacity(total_len);
            for part in parts.into_iter().rev() {
                buffer.push_str(part);
            }
            assert_eq!(buffer.len(), total_len);
            buffer
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.state.size_hint()
    }
}

#[derive(Clone, Debug)]
struct FlattenGroupChildren<'a> {
    remaining_children: std::slice::Iter<'a, GlobExpr>,
    current_child: Option<ExpandState<'a>>,
}
impl<'a> Iterator for FlattenGroupChildren<'a> {
    type Item = ExpandBuffer<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut child) = self.current_child {
                if let Some(item) = child.next() {
                    break Some(item);
                }
            }
            // current child is None or empty, get next child
            match self.remaining_children.next() {
                Some(child) => {
                    self.current_child = Some(ExpandState::new(child));
                }
                // iterator is empty & remaining are empty
                None => break None,
            }
        }
    }
}
#[derive(Clone, Debug)]
enum ExpandState<'a> {
    Finished,
    Literal(&'a Text),
    Group {
        prefix: &'a Text,
        product: Box<Product<FlattenGroupChildren<'a>, ExpandState<'a>>>,
    },
    /// Temporary state used for updates
    Invalid,
}
impl<'a> ExpandState<'a> {
    fn new(expr: &'a GlobExpr) -> Self {
        match expr.kind {
            GlobExprKind::Literal(ref literal) => ExpandState::Literal(literal),
            GlobExprKind::Group(ref group) => ExpandState::Group {
                prefix: match group.prefix.kind {
                    GlobExprKind::Literal(ref lit) => lit,
                    _ => panic!("Invalid state, prefix must be literal: {:?}", group.prefix),
                },
                product: Box::new(
                    FlattenGroupChildren {
                        remaining_children: group.children.iter(),
                        current_child: None,
                    }
                    .cartesian_product(ExpandState::new(&group.suffix)),
                ),
            },
        }
    }
}
impl<'a> Iterator for ExpandState<'a> {
    type Item = ExpandBuffer<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        fn finished<'b>(state: &mut ExpandState<'b>) -> Option<ExpandBuffer<'b>> {
            *state = ExpandState::Finished;
            None
        }
        let old_state = std::mem::replace(self, ExpandState::Invalid);
        let (new_state, item) = match old_state {
            ExpandState::Invalid => unreachable!(),
            ExpandState::Finished => return finished(self), // state unchanged, no items
            ExpandState::Literal(item) => (ExpandState::Finished, smallvec::smallvec_inline![item]),
            ExpandState::Group {
                prefix,
                mut product,
            } => {
                let Some((child_parts, mut suffix_parts)) = product.next() else {
                    return finished(self);
                };
                suffix_parts.extend(child_parts);
                suffix_parts.push(prefix);
                (ExpandState::Group { prefix, product }, suffix_parts)
            }
        };
        *self = new_state;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact_len = match self {
            ExpandState::Finished => 0,
            ExpandState::Literal(_) => 1,
            ExpandState::Group { .. } => {
                return (0, None); // can't give meaningful hint
            }
            ExpandState::Invalid => unreachable!(),
        };
        (exact_len, Some(exact_len))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GlobGroup {
    pub prefix: GlobExpr,
    pub children: Vec<GlobExpr>,
    pub suffix: GlobExpr,
}
impl GlobGroup {
    pub fn into_expr(self) -> GlobExpr {
        GlobExpr {
            kind: GlobExprKind::Group(Box::new(self)),
        }
    }
}

/// An in-progress expansion
///
/// This stores a single element inline to avoid allocations.
type ExpandBuffer<'a> = smallvec::SmallVec<&'a Text, 1>;
