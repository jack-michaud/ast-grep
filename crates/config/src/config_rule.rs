use ast_grep_core::language::Language;
use ast_grep_core::meta_var::MetaVarEnv;
use ast_grep_core::ops as o;
use ast_grep_core::{KindMatcher, Matcher, Node, Pattern};
use serde::{Deserialize, Serialize};
use std::pin::Pin;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum SerializableRule {
    All(Vec<SerializableRule>),
    Any(Vec<SerializableRule>),
    Not(Box<SerializableRule>),
    Inside(Box<SerializableRule>),
    Has(Box<SerializableRule>),
    Pattern(PatternStyle),
    Kind(String),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum PatternStyle {
    Str(String),
    Contextual {
        context: String,
        selector: String,
    }
}


pub enum DynamicRule<L: Language + 'static> {
    All(o::All<L, DynamicRule<L>>),
    Any(o::Any<L, DynamicRule<L>>),
    Not(Box<o::Not<L, DynamicRule<L>>>),
    Inside(Box<o::Inside<L, DynamicRule<L>>>),
    Has(Box<o::Has<L, DynamicRule<L>>>),
    Pattern(Pin<Box<Pattern<L>>>),
    Kind(KindMatcher<L>),
}

impl<L: Language> Matcher<L> for DynamicRule<L> {
    fn match_node_with_env<'tree>(
        &self,
        node: Node<'tree, L>,
        env: &mut MetaVarEnv<'tree, L>,
    ) -> Option<ast_grep_core::Node<'tree, L>> {
        use DynamicRule::*;
        match self {
            All(all) => all.match_node_with_env(node, env),
            Any(any) => any.match_node_with_env(node, env),
            Not(not) => not.match_node_with_env(node, env),
            Inside(inside) => inside.match_node_with_env(node, env),
            Has(has) => has.match_node_with_env(node, env),
            Pattern(pattern) => pattern.match_node_with_env(node, env),
            Kind(kind) => kind.match_node_with_env(node, env),
        }
    }
}

enum SerializeError {
    MissPositiveMatcher,
}

// TODO: implement positive/non positive
pub fn from_serializable<L: Language>(serialized: SerializableRule, lang: L) -> DynamicRule<L> {
    use DynamicRule as D;
    use SerializableRule as S;
    let mapper = |s| from_serializable(s, lang);
    match serialized {
        S::All(all) => D::All(o::All::new(all.into_iter().map(mapper))),
        S::Any(any) => D::Any(o::Any::new(any.into_iter().map(mapper))),
        S::Not(not) => D::Not(Box::new(o::Not::new(mapper(*not)))),
        S::Inside(inside) => D::Inside(Box::new(o::Inside::new(mapper(*inside)))),
        S::Has(has) => D::Has(Box::new(o::Has::new(mapper(*has)))),
        S::Pattern(PatternStyle::Str(pattern)) => D::Pattern(Pattern::new(&pattern, lang)),
        S::Pattern(PatternStyle::Contextual { .. }) => todo!(),
        S::Kind(kind) => D::Kind(KindMatcher::new(&kind, lang)),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_yaml::from_str;
    use SerializableRule::*;
    use PatternStyle::*;

    #[test]
    fn test_pattern() {
        let src = r"
pattern: Test
";
        let rule: SerializableRule = from_str(src).expect("cannot parse rule");
        assert!(matches!(rule, Pattern(Str(_))));
        let src = r"
pattern:
    context: class $C { set $B() {} }
    selector: method_definition
";
        let rule: SerializableRule = from_str(src).expect("cannot parse rule");
        assert!(matches!(rule, Pattern(Contextual {..})));
    }
}
