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
    tokens::{Brace, IdentToken, Repeated, ToTokens},
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

    /// Merge struct fields with warning emission for conflicts
    pub fn merge_struct_with_warnings(
        &mut self,
        source: PathBuf,
        source_content: Option<&Arc<String>>,
        operand_name: &str,
        fields: Vec<crate::tokens::RepeatedItem<Arg, Token![,]>>,
        union_span: Option<crate::Span>,
    ) {
        for field in fields {
            let field_name = field
                .value
                .value
                .name
                .borrow_string()
                .clone();
            let field_type = &field.value.value.typ;

            match self.fields.entry(field_name.clone()) {
                std::collections::btree_map::Entry::Occupied(existing) => {
                    let existing_type = &existing.get().0.value.value.typ;
                    let span = crate::Span::new(field.value.span().start, field.value.span().end);

                    // Get type strings for error messages
                    let cfg = crate::fmt::FormatConfig::default();
                    let existing_type_str = {
                        let mut printer = crate::fmt::Printer::new(&cfg);
                        existing_type.write(&mut printer);
                        printer.buf
                    };
                    let new_type_str = {
                        let mut printer = crate::fmt::Printer::new(&cfg);
                        field_type.write(&mut printer);
                        printer.buf
                    };

                    let types_same = existing_type_str == new_type_str;

                    // Build diagnostic with primary span (field) and optional secondary span (union)
                    let build_warning = |error: kintsu_errors::CompilerError| {
                        let mut diag = kintsu_events::Diagnostic::from(error);
                        if let Some(content) = source_content {
                            diag = diag.with_source(
                                source.display().to_string(),
                                std::sync::Arc::from(content.as_str()),
                            );
                        }
                        if let Some(u_span) = union_span {
                            diag = diag.add_label(u_span, "in this union");
                        }
                        diag
                    };

                    if types_same {
                        // KUN8001: Field shadowed (same type)
                        let error: kintsu_errors::CompilerError =
                            crate::UnionError::field_shadowed(
                                &field_name,
                                operand_name,
                                &existing_type_str,
                            )
                            .at(span)
                            .build();
                        kintsu_events::emit_warning(build_warning(error));
                    } else {
                        // KUN3001: Field conflict (different types)
                        let error: kintsu_errors::CompilerError =
                            crate::UnionError::field_conflict(
                                &field_name,
                                &existing_type_str,
                                &new_type_str,
                            )
                            .at(span)
                            .build();
                        kintsu_events::emit_warning(build_warning(error));
                    }
                },
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert((field.value.with_source(source.clone()), field.sep.clone()));
                },
            }
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
