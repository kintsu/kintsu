use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use crate::{
    Token,
    ast::{
        anonymous::AnonymousStruct,
        items::StructDef,
        strct::{Arg, Struct},
        ty::Type,
        union::Union,
    },
    ctx::{
        SourceSpanned,
        common::{FromNamedSource, WithSource},
    },
    defs::Spanned,
    tokens::{Brace, IdentToken, Repeated},
};

pub fn build_struct_def_from_anonymous(
    generated_name: String,
    anonymous: AnonymousStruct,
    source: PathBuf,
) -> FromNamedSource<StructDef> {
    let args = Repeated {
        values: anonymous.fields.value.values,
    };

    StructDef {
        // todo: pass parent version info
        meta: Vec::new(),
        def: Spanned::call_site(Struct {
            kw: Spanned::call_site(<Token![struct]>::new()),
            name: Spanned::call_site(IdentToken::new(generated_name)),
            brace: anonymous.brace.clone(),
            args,
        }),
        end: Spanned::call_site(<Token![;]>::new()),
    }
    .with_source(source)
}

#[derive(Clone)]
pub struct UnionRecord {
    pub union_ref: Arc<Spanned<Union>>,
    pub context_stack: Vec<String>,

    pub in_oneof: bool,
    /// if in_oneof, the variant index for numeric suffix
    pub variant_index: Option<usize>,
}

impl UnionRecord {
    pub fn generate_name(&self) -> String {
        use convert_case::{Case, Casing};

        let base_name = self
            .context_stack
            .join("_")
            .to_case(Case::Pascal);

        if self.in_oneof {
            if let Some(idx) = self.variant_index {
                format!("{}{}", base_name, idx)
            } else {
                base_name
            }
        } else {
            base_name
        }
    }
}

#[derive(Clone)]
pub struct AliasEntry {
    pub target_type: Spanned<Type>,
    pub source: PathBuf,
    pub dependencies: Vec<String>,
}

pub struct UnionWorkingSet {
    fields: BTreeMap<String, (SourceSpanned<Arg>, Option<Spanned<Token![,]>>)>,
}

impl UnionWorkingSet {
    pub fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
        }
    }

    pub fn merge_struct(
        &mut self,
        source: PathBuf,
        fields: Vec<crate::tokens::RepeatedItem<Arg, Token![,]>>,
    ) {
        for field in fields {
            let field_name = field
                .value
                .value
                .name
                .borrow_string()
                .clone();

            self.fields
                .entry(field_name)
                .or_insert_with(|| (field.value.with_source(source.clone()), field.sep.clone()));
        }
    }

    pub fn into_struct_def(
        self,
        generated_name: String,
        source: PathBuf,
        brace: Brace,
    ) -> FromNamedSource<StructDef> {
        let args: Vec<crate::tokens::RepeatedItem<_, Token![,]>> = self
            .fields
            .into_values()
            .map(|(from_source, sep)| {
                crate::tokens::RepeatedItem {
                    value: from_source.into_spanned(),
                    sep,
                }
            })
            .collect();

        StructDef {
            meta: Vec::new(),
            def: Spanned::call_site(Struct {
                kw: Spanned::call_site(<Token![struct]>::new()),
                name: Spanned::call_site(IdentToken::new(generated_name)),
                brace,
                args: Repeated { values: args },
            }),
            end: Spanned::call_site(<Token![;]>::new()),
        }
        .with_source(source)
    }
}

#[derive(Default)]
pub struct NameContext {
    pub stack: Vec<String>,
}

impl NameContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(
        &mut self,
        name: impl Into<String>,
    ) {
        self.stack.push(name.into());
    }

    pub fn pop(&mut self) {
        self.stack.pop();
    }

    pub fn generate_name(&self) -> String {
        use convert_case::{Case, Casing};
        self.stack.join("_").to_case(Case::Pascal)
    }
}
