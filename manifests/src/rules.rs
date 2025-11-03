use std::{collections::BTreeMap, hash::Hash};

#[derive(PartialEq, PartialOrd, serde::Serialize, serde::Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum RuleLevel {
    Silent,
    Info,
    #[default]
    Warn,
    Error,
}

#[derive(PartialEq, PartialOrd, serde::Serialize, serde::Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Fix {
    Skip,
    #[default]
    Unsafe,
    Safe,
}

impl Fix {
    pub fn diagnostic(&self) -> &'static str {
        match self {
            Self::Skip => "info: this fix is being skipped",
            Self::Safe => "info: safe to fix",
            Self::Unsafe => "warning: warning: this fix is unsafe and requires the `--unsafe` flag",
        }
    }
}

pub trait WithRule {
    fn with_level(
        &mut self,
        level: RuleLevel,
    );

    fn with_fix(
        &mut self,
        fix: Fix,
    );

    fn with(
        &mut self,
        overrides: &RuleOverrides,
    ) {
        if let Some(level) = &overrides.level {
            self.with_level(level.clone())
        }
        if let Some(fix) = &overrides.fix {
            self.with_fix(fix.clone())
        }
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct RuleOverrides {
    #[serde(default)]
    level: Option<RuleLevel>,
    #[serde(default)]
    fix: Option<Fix>,
}

#[derive(serde::Deserialize, validator::Validate)]
pub struct RuleConfig<RuleGroup: Ord + Hash> {
    #[serde(default = "BTreeMap::new")]
    pub overrides: BTreeMap<RuleGroup, BTreeMap<String, RuleOverrides>>,
}

impl<RuleGroup: Ord + Hash> Default for RuleConfig<RuleGroup> {
    fn default() -> Self {
        Self {
            overrides: BTreeMap::new(),
        }
    }
}

#[macro_export]
macro_rules! rule_config {
    ($name: literal for $group_ty: ty $({$($cfg: ident: $t: ty), + $(,)?})?) => {
        #[derive(serde::Deserialize, Default, validator::Validate)]
        pub struct RuleConfig {
            #[serde(flatten, default)]
            config: $crate::rules::RuleConfig<$group_ty>,
            $($(
                #[serde(flatten, default)]
                $cfg: $t,
            )*)*
        }

        impl RuleConfig {
            pub fn new() -> Self {
                Self::default()
            }
        }

        impl AsRef<$crate::rules::RuleConfig<$group_ty>> for RuleConfig {
            fn as_ref(&self) -> &$crate::rules::RuleConfig<$group_ty> {
                &self.config
            }
        }

        impl std::ops::Deref for RuleConfig {
            type Target = $crate::rules::RuleConfig<$group_ty>;

            fn deref(&self) -> &Self::Target {
                &self.config
            }
        }

        impl $crate::config::NewForConfig for RuleConfig {
            const NAME: &'static str = $name;
        }
    };
}
