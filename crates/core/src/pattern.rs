use crate::language::Language;
use crate::match_tree::match_node_non_recursive;
use crate::matcher::{Matcher, PositiveMatcher};
use crate::{meta_var::MetaVarEnv, Node, Root};

use std::pin::Pin;
use std::marker::PhantomPinned;

#[derive(Clone)]
pub struct Pattern<L: Language> {
    pub root: Root<L>,
    node: *const tree_sitter::Node<'static>,
    _marker: PhantomPinned,
}

unsafe impl<L: Language> Sync for Pattern<L> {}

impl<L: Language + 'static> Pattern<L> {
    pub fn new(src: &str, lang: L) -> Pin<Box<Self>> {
        let root = Root::new(src, lang);
        let goal = root.root().inner;
        if goal.child_count() != 1 {
            todo!("multi-children pattern is not supported yet.")
        }
        let t = Self {
            root,
            node: std::ptr::null(),
            _marker: PhantomPinned,
        };
        let mut boxed = Box::pin(t);
        let root = &boxed.as_ref().root;
        let mut node = root.root().inner;
        while node.child_count() == 1 {
            node = node.child(0).unwrap();
        }
        let node_ref: *const tree_sitter::Node = unsafe {
            std::mem::transmute(&node)
        };
        unsafe { boxed.as_mut().get_unchecked_mut().node = node_ref };
        boxed
    }
}

impl<L: Language> Matcher<L> for Pattern<L> {
    fn match_node_with_env<'tree>(
        &self,
        node: Node<'tree, L>,
        env: &mut MetaVarEnv<'tree, L>,
    ) -> Option<Node<'tree, L>> {
        match_node_non_recursive(&matcher(&self.root), node, env)
    }
}

impl<L: Language> std::fmt::Debug for Pattern<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", matcher(&self.root).inner.to_sexp())
    }
}

// TODO: extract out matcher in recursion
fn matcher<L: Language>(goal: &Root<L>) -> Node<L> {
    let mut node = goal.root().inner;
    while node.child_count() == 1 {
        node = node.child(0).unwrap();
    }
    let goal = Node {
        inner: node,
        root: goal,
    };
    goal
}

impl<L: Language> PositiveMatcher<L> for Pattern<L> {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::language::Tsx;
    use std::collections::HashMap;

    fn pattern_node(s: &str) -> Root<Tsx> {
        Root::new(s, Tsx)
    }

    fn test_match(s1: &str, s2: &str) {
        let pattern = Pattern::new(s1, Tsx);
        let goal = pattern_node(s1);
        let cand = pattern_node(s2);
        let cand = cand.root();
        assert!(
            pattern.find_node(cand).is_some(),
            "goal: {}, candidate: {}",
            goal.root().inner.to_sexp(),
            cand.inner.to_sexp(),
        );
    }
    fn test_non_match(s1: &str, s2: &str) {
        let pattern = Pattern::new(s1, Tsx);
        let goal = pattern_node(s1);
        let cand = pattern_node(s2);
        let cand = cand.root();
        assert!(
            pattern.find_node(cand).is_none(),
            "goal: {}, candidate: {}",
            goal.root().inner.to_sexp(),
            cand.inner.to_sexp(),
        );
    }

    #[test]
    fn test_meta_variable() {
        test_match("const a = $VALUE", "const a = 123");
        test_match("const $VARIABLE = $VALUE", "const a = 123");
        test_match("const $VARIABLE = $VALUE", "const a = 123");
    }

    fn match_env(goal_str: &str, cand: &str) -> HashMap<String, String> {
        let pattern = Pattern::new(goal_str, Tsx);
        let cand = pattern_node(cand);
        let cand = cand.root();
        let mut env = MetaVarEnv::new();
        pattern.find_node_with_env(cand, &mut env).unwrap();
        HashMap::from(env)
    }

    #[test]
    fn test_meta_variable_env() {
        let env = match_env("const a = $VALUE", "const a = 123");
        assert_eq!(env["VALUE"], "123");
    }

    #[test]
    fn test_match_non_atomic() {
        let env = match_env("const a = $VALUE", "const a = 5 + 3");
        assert_eq!(env["VALUE"], "5 + 3");
    }

    #[test]
    fn test_class_assignment() {
        test_match("class $C { $MEMBER = $VAL}", "class A {a = 123}");
        test_non_match("class $C { $MEMBER = $VAL; b = 123; }", "class A {a = 123}");
        // test_match("a = 123", "class A {a = 123}");
        test_non_match("a = 123", "class B {b = 123}");
    }

    #[test]
    fn test_return() {
        test_match("$A($B)", "return test(123)");
    }
}
