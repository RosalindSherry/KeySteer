//! Bounded containing-window lookup, independent of native accessibility objects.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Link {
    Window,
    TopLevel,
    Parent,
}

pub(crate) trait Lookup {
    type Node;
    fn expired(&self) -> bool;
    fn same(&self, left: &Self::Node, right: &Self::Node) -> bool;
    fn usable(&self, node: &Self::Node) -> bool;
    fn related(&self, node: &Self::Node, link: Link) -> Option<Self::Node>;
}

pub(crate) fn resolve<L: Lookup>(lookup: &L, hit: L::Node) -> Option<L::Node> {
    fn visit<L: Lookup>(
        lookup: &L,
        node: L::Node,
        seen: &mut smallvec::SmallVec<[L::Node; 8]>,
    ) -> Option<L::Node> {
        if lookup.expired() || seen.len() >= 16 || seen.iter().any(|old| lookup.same(old, &node)) {
            return None;
        }
        if lookup.usable(&node) {
            return Some(node);
        }
        let index = seen.len();
        seen.push(node);
        for link in [Link::Window, Link::TopLevel, Link::Parent] {
            if lookup.expired() || seen.len() >= 16 {
                break;
            }
            if let Some(next) = lookup.related(&seen[index], link)
                && let Some(window) = visit(lookup, next, seen)
            {
                return Some(window);
            }
        }
        None
    }
    visit(lookup, hit, &mut smallvec::SmallVec::new())
}

/// Missing subrole metadata alone does not invalidate a real AX window.
pub(crate) fn ordinary_role(role: Option<&str>, subrole: Option<&str>) -> bool {
    role == Some("AXWindow") && matches!(subrole, None | Some("AXStandardWindow"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    struct Graph {
        edges: Vec<[Option<usize>; 3]>,
        target: Option<usize>,
        calls: Cell<usize>,
        limit: usize,
    }
    impl Lookup for Graph {
        type Node = usize;
        fn expired(&self) -> bool {
            self.calls.get() >= self.limit
        }
        fn same(&self, a: &usize, b: &usize) -> bool {
            a == b
        }
        fn usable(&self, node: &usize) -> bool {
            Some(*node) == self.target
        }
        fn related(&self, node: &usize, link: Link) -> Option<usize> {
            self.calls.set(self.calls.get() + 1);
            self.edges[*node][match link {
                Link::Window => 0,
                Link::TopLevel => 1,
                Link::Parent => 2,
            }]
        }
    }
    #[test]
    fn missing_or_invalid_window_link_uses_top_level_and_parents() {
        let graph = Graph {
            edges: vec![
                [Some(1), Some(2), None],
                [Some(1), None, None],
                [None, None, Some(3)],
                [None; 3],
            ],
            target: Some(3),
            calls: Cell::new(0),
            limit: 99,
        };
        assert_eq!(resolve(&graph, 0), Some(3));
        // The unusable direct window must not prevent the sheet/parent fallback.
        assert!(graph.calls.get() < 15);
    }
    #[test]
    fn valid_hit_avoids_relationship_queries() {
        let graph = Graph {
            edges: vec![[None; 3]],
            target: Some(0),
            calls: Cell::new(0),
            limit: 99,
        };
        assert_eq!(resolve(&graph, 0), Some(0));
        assert_eq!(graph.calls.get(), 0);
    }
    #[test]
    fn cycles_depth_and_deadline_are_bounded() {
        let mut graph = Graph {
            edges: (0..40).map(|n| [None, None, Some((n + 1) % 40)]).collect(),
            target: Some(30),
            calls: Cell::new(0),
            limit: 999,
        };
        assert_eq!(resolve(&graph, 0), None);
        assert!(graph.calls.get() <= 48);
        graph.calls.set(0);
        graph.limit = 2;
        assert_eq!(resolve(&graph, 0), None);
        assert_eq!(graph.calls.get(), 2);
    }
    #[test]
    fn missing_subrole_is_allowed_but_known_nonstandard_roles_are_not() {
        assert!(ordinary_role(Some("AXWindow"), None));
        assert!(ordinary_role(Some("AXWindow"), Some("AXStandardWindow")));
        assert!(!ordinary_role(Some("AXWindow"), Some("AXDialog")));
        assert!(!ordinary_role(Some("AXSheet"), None));
        assert!(!ordinary_role(Some("AXButton"), None));
        assert!(!ordinary_role(None, None));
    }
}
