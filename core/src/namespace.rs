use std::collections::BTreeMap;

use convert_case::Casing;

use crate::{
    Contiguous, Definitions, Enum, ErrorTy, Field, FieldOrRef, Ident, OneOf, Operation, Struct,
    Version,
};

#[cfg(feature = "generate")]
use crate::generate::LanguageTrait;

pub trait OfNamespace {
    const NAMESPACE: &'static str;
}

#[macro_export]
macro_rules! namespace {
    ($ns: literal {
        $($t: path), + $(,)?
    }) => {
        $(
            impl $crate::namespace::OfNamespace for $t {
                const NAMESPACE: &'static str = $ns;
            }
        )*
    };
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, bon::Builder, Clone)]
pub struct Namespace {
    pub name: Ident,
    pub version: Version,

    pub fields: BTreeMap<Ident, Field<Ident>>,
    pub ops: BTreeMap<Ident, Operation>,
    pub defs: BTreeMap<Ident, Struct>,
    pub enums: BTreeMap<Ident, Enum>,
    pub one_ofs: BTreeMap<Ident, OneOf>,
    pub errors: BTreeMap<Ident, ErrorTy>,
}

impl Namespace {
    pub fn new<I: Into<Ident>>(name: I) -> Self {
        Self {
            name: name.into(),
            version: 1_usize.into(),
            fields: Default::default(),
            ops: Default::default(),
            defs: Default::default(),
            enums: Default::default(),
            one_ofs: Default::default(),
            errors: Default::default(),
        }
    }

    pub fn with_definitions(
        &mut self,
        defs: Vec<Definitions>,
    ) -> crate::Result<()> {
        for def in defs {
            self.with_definition(def)?;
        }
        Ok(())
    }

    pub fn with_definition(
        &mut self,
        def: Definitions,
    ) -> crate::Result<()> {
        match def {
            Definitions::FieldV1(field) => {
                unique_ns_def(
                    &mut self.fields,
                    field.meta.name.clone(),
                    &self.name,
                    field,
                    "field",
                )?;
            },
            Definitions::StructV1(def) => {
                unique_ns_def(
                    &mut self.defs,
                    def.meta.name.clone(),
                    &self.name,
                    def,
                    "struct",
                )?;
            },
            Definitions::OperationV1(op) => {
                unique_ns_def(
                    &mut self.ops,
                    op.meta.name.clone(),
                    &self.name,
                    op,
                    "operation",
                )?;
            },
            Definitions::EnumV1(enm) => {
                unique_ns_def(
                    &mut self.enums,
                    enm.meta.name.clone(),
                    &self.name,
                    enm,
                    "enum",
                )?;
            },
            Definitions::OneOfV1(one) => {
                unique_ns_def(
                    &mut self.one_ofs,
                    one.meta.name.clone(),
                    &self.name,
                    one,
                    "one_of",
                )?;
            },
            Definitions::ErrorV1(err) => {
                unique_ns_def(
                    &mut self.errors,
                    err.meta.name.clone(),
                    &self.name,
                    err,
                    "error",
                )?;
            },
            // note: this is an internal error
            Definitions::NamespaceV1(ns) => {
                return Err(crate::Error::NamespaceConflict {
                    name: ns.name,
                    tag: "nameespace",
                    ns: self.name.clone(),
                });
            },
        }

        Ok(())
    }

    pub fn resolve_field(
        &self,
        name: &Ident,
    ) -> crate::Result<&Field<Ident>> {
        self.fields.get(name).ok_or_else(|| {
            crate::Error::NameNotFound {
                name: name.clone(),
                ns: self.name.clone(),
            }
        })
    }

    pub fn resolve_struct(
        &self,
        name: &Ident,
    ) -> crate::Result<&Struct> {
        self.defs.get(name).ok_or_else(|| {
            crate::Error::NameNotFound {
                name: name.clone(),
                ns: self.name.clone(),
            }
        })
    }

    pub fn resolve_enum(
        &self,
        name: &Ident,
    ) -> crate::Result<&Enum> {
        self.enums.get(name).ok_or_else(|| {
            crate::Error::NameNotFound {
                name: name.clone(),
                ns: self.name.clone(),
            }
        })
    }

    pub fn resolve_field_types(&mut self) -> crate::Result<()> {
        for (def_name, def) in self.defs.clone().iter() {
            let mut swap = vec![];
            for (field_name, field) in &*def.fields {
                match field {
                    FieldOrRef::Value(..) => {},
                    FieldOrRef::Ref { to } => {
                        if let Some(local_ref) = def.fields.get(to) {
                            swap.push((field_name.clone(), local_ref.clone()));
                        } else {
                            swap.push((
                                field_name.clone(),
                                FieldOrRef::Value(self.resolve_field(to)?.clone().into()),
                            ));
                        }
                    },
                }
            }

            for (name, field) in swap {
                self.defs
                    .get_mut(def_name)
                    .unwrap()
                    .fields
                    .insert(name, field);
            }
        }

        Ok(())
    }

    pub fn ensure_contiguousness(&mut self) -> crate::Result<()> {
        for (ident, desc) in self.enums.iter() {
            desc.variants.is_contiguous(ident)?;
        }
        Ok(())
    }

    pub fn check(&mut self) -> crate::Result<()> {
        self.simplify_types();
        self.resolve_field_types()?;
        self.ensure_contiguousness()?;
        Ok(())
    }

    #[cfg(feature = "generate")]
    pub fn normalized_path<L: LanguageTrait>(&self) -> String {
        self.name
            .to_string()
            .replace(".", "_")
            .to_case(L::file_case())
    }

    pub fn simplify_types(&mut self) {
        for ty in self.fields.values_mut() {
            ty.ty = ty.ty.simplify();
        }

        for def in self.defs.values_mut() {
            for field in def.fields.values_mut() {
                if let FieldOrRef::Value(f) = field {
                    f.ty = f.ty.simplify();
                }
            }
        }

        for one in self.one_ofs.values_mut() {
            for variant in one.variants.values_mut() {
                variant.ty = variant.ty.simplify();
            }
        }
    }

    pub fn merge(
        &mut self,
        other: Self,
    ) -> crate::Result<()> {
        for def in other.defs.into_values() {
            self.with_definition(Definitions::StructV1(def))?;
        }
        for enums in other.enums.into_values() {
            self.with_definition(Definitions::EnumV1(enums))?;
        }
        for one in other.one_ofs.into_values() {
            self.with_definition(Definitions::OneOfV1(one))?;
        }
        for fields in other.fields.into_values() {
            self.with_definition(Definitions::FieldV1(fields))?;
        }
        for op in other.ops.into_values() {
            self.with_definition(Definitions::OperationV1(op))?;
        }
        Ok(())
    }
}

#[inline]
pub(crate) fn unique_ns_def<T>(
    sources: &mut BTreeMap<Ident, T>,
    name: Ident,
    ns: &Ident,
    def: T,
    tag: &'static str,
) -> crate::Result<()> {
    match sources.insert(name.clone(), def) {
        Some(..) => {
            Err(crate::Error::NamespaceConflict {
                ns: ns.clone(),
                name,
                tag,
            })
        },
        None => Ok(()),
    }
}
