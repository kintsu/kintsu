use kintsu_manifests::rules::*;
use serde::Deserialize;

use crate::{OneOf, Struct, Type};

pub mod self_ref;

#[allow(unused_variables)]
pub trait Check: Send + Sync {
    fn check_type(
        &self,
        ty: &Type,
    ) -> crate::Result<()> {
        Ok(())
    }
    fn check_struct(
        &self,
        def: &Struct,
    ) -> crate::Result<()> {
        Ok(())
    }

    fn check_one_of(
        &self,
        one_of: &OneOf,
    ) -> crate::Result<()> {
        Ok(())
    }
}

#[derive(Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RuleGroup {
    Form,
}

kintsu_manifests::rule_config! {
    "op-check" for RuleGroup
}

dyn_inventory::dyn_inventory! {
    Rule<Handle: Check> {
        pub group: RuleGroup,
        pub name: &'static str,
        pub level: RuleLevel,
        pub description: &'static str,

        pub fix: Fix,
        pub handle: Handle,
    }
}

#[allow(unused)]
pub struct RuleRegistry {
    collector: RuleCollector,
    config: RuleConfig,
}

impl WithRule for Rule {
    fn with_fix(
        &mut self,
        fix: Fix,
    ) {
        self.fix = fix;
    }

    fn with_level(
        &mut self,
        level: RuleLevel,
    ) {
        self.level = level;
    }
}

impl RuleRegistry {
    pub fn new(config: RuleConfig) -> Self {
        let collector = RuleCollector::new_with(|rule| {
            if let Some(overrides) = &config.overrides.get(&rule.group)
                && let Some(rule_set) = overrides.get(rule.name)
            {
                rule.with(rule_set);
            }
        });

        Self { collector, config }
    }

    // pub async fn run(&self) -> crate::Result<()> {
    //     let mut futs = vec![];
    //     for plugin in &self.collector.plugins {

    //     }
    // }
}

#[macro_export]
macro_rules! rule {
    (
       $name: ident in $group: ident @ $level: ident : $fix: ident; $desc: literal
    ) => {
        dyn_inventory::emit!(
            $name Check as Rule {
                group = $crate::checks::RuleGroup::$group,
                name = stringify!($name),
                level = $crate::checks::RuleLevel::$level,
                description = $desc,
                fix = $crate::checks::Fix::$fix,
            }
        );
    };
}

#[cfg(test)]
mod test {
    use super::*;

    crate::rule!(Fails in Form @ Error: Safe; "rule abc: must be xyz");

    impl Check for Fails {
        fn check_type(
            &self,
            _: &Type,
        ) -> crate::Result<()> {
            Err(crate::Error::NamespaceConflict {
                name: "test".into(),
                tag: "tag",
                ns: "some-ns".into(),
            })
        }
    }

    #[test_case::test_case("Fails", |check| {
        assert!{
            check.check_type(&Type::Binary).is_err()
        }
    }; "test plugin loads" )]
    fn test_plugin<F: Fn(Box<dyn Check>)>(
        name: &'static str,
        f: F,
    ) {
        let registry = super::RuleRegistry::new(RuleConfig::default());

        for plugin in registry.collector.plugins {
            if plugin.name == name {
                f(plugin.handle);
                return;
            }
        }

        panic!("did not run {name}")
    }
}
