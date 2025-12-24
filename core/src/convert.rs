//! Conversions between core types and parser declaration types.
//!
//! This module bridges the legacy core type system with the unified parser types.

use crate::{
    CompoundType, Definitions, Enum, ErrorTy, Field, FieldOrRef, Ident, Meta, OneOf, OneOfVariant,
    Operation, StrOrInt, Struct, Type, Version,
    declare::{
        Builtin, DeclArg, DeclComment, DeclEnum, DeclEnumDef, DeclError, DeclField, DeclIntVariant,
        DeclMeta, DeclNamespace, DeclOneOf, DeclOneOfVariant, DeclOperation, DeclStringVariant,
        DeclStruct, DeclType, TypeDefinition,
    },
    namespace::Namespace,
};

fn doc_to_comment(doc: &Option<String>) -> DeclComment {
    doc.as_ref()
        .map(|d| DeclComment::from_vec(vec![d.clone()]))
        .unwrap_or_default()
}

impl From<&Type> for DeclType {
    fn from(ty: &Type) -> Self {
        match ty {
            Type::U8 => DeclType::Builtin { ty: Builtin::U8 },
            Type::U16 => DeclType::Builtin { ty: Builtin::U16 },
            Type::U32 => DeclType::Builtin { ty: Builtin::U32 },
            Type::U64 => DeclType::Builtin { ty: Builtin::U64 },
            Type::Usize => DeclType::Builtin { ty: Builtin::Usize },
            Type::I8 => DeclType::Builtin { ty: Builtin::I8 },
            Type::I16 => DeclType::Builtin { ty: Builtin::I16 },
            Type::I32 => DeclType::Builtin { ty: Builtin::I32 },
            Type::I64 => DeclType::Builtin { ty: Builtin::I64 },
            Type::F32 => DeclType::Builtin { ty: Builtin::F32 },
            Type::F64 => DeclType::Builtin { ty: Builtin::F64 },
            Type::Bool => DeclType::Builtin { ty: Builtin::Bool },
            Type::String => DeclType::Builtin { ty: Builtin::Str },
            Type::DateTime => {
                DeclType::Builtin {
                    ty: Builtin::DateTime,
                }
            },
            Type::Complex => {
                DeclType::Builtin {
                    ty: Builtin::Complex,
                }
            },
            Type::Binary => {
                DeclType::Builtin {
                    ty: Builtin::Binary,
                }
            },
            Type::Never => DeclType::Builtin { ty: Builtin::Never },
            Type::CompoundType(ct) => ct.into(),
        }
    }
}

impl From<Type> for DeclType {
    fn from(ty: Type) -> Self {
        (&ty).into()
    }
}

impl From<&CompoundType> for DeclType {
    fn from(ct: &CompoundType) -> Self {
        match ct {
            CompoundType::Option { ty } => {
                DeclType::Optional {
                    inner_type: Box::new(ty.as_ref().into()),
                }
            },
            CompoundType::Array { ty } => {
                DeclType::Array {
                    element_type: Box::new(ty.as_ref().into()),
                }
            },
            CompoundType::SizedArray { size, ty } => {
                DeclType::SizedArray {
                    element_type: Box::new(ty.as_ref().into()),
                    size: *size as u64,
                }
            },
            CompoundType::Enum { to }
            | CompoundType::Struct { to }
            | CompoundType::OneOf { to } => {
                DeclType::Named {
                    reference: crate::declare::DeclNamedItemContext {
                        context: crate::declare::DeclRefContext {
                            package: String::new(),
                            namespace: vec![],
                        },
                        name: to.to_string(),
                    },
                }
            },
            CompoundType::Union { lhs, rhs: _ } => {
                DeclType::Paren {
                    inner_type: Box::new(lhs.as_ref().into()),
                }
            },
        }
    }
}

impl<NsTy: ToString, IdentTy: ToString, Ver> From<&Meta<IdentTy, NsTy, Ver>> for DeclMeta
where
    Ver: Into<u32> + Copy,
{
    fn from(_meta: &Meta<IdentTy, NsTy, Ver>) -> Self {
        DeclMeta { version: 1 }
    }
}

impl From<&Field<Ident>> for DeclField {
    fn from(field: &Field<Ident>) -> Self {
        DeclField {
            name: field.meta.name.to_string(),
            ty: (&field.ty).into(),
            default_value: None,
            optional: field.optional,
            comments: DeclComment::default(),
        }
    }
}

impl From<&Field<Option<Ident>>> for DeclField {
    fn from(field: &Field<Option<Ident>>) -> Self {
        DeclField {
            name: field
                .meta
                .name
                .as_ref()
                .map(|i| i.to_string())
                .unwrap_or_default(),
            ty: (&field.ty).into(),
            default_value: None,
            optional: field.optional,
            comments: DeclComment::default(),
        }
    }
}

impl From<&Struct> for DeclStruct {
    fn from(s: &Struct) -> Self {
        let fields: Vec<DeclField> = s
            .fields
            .iter()
            .filter_map(|(name, field_or_ref)| {
                match field_or_ref {
                    FieldOrRef::Value(f) => {
                        let mut df: DeclField = f.into();
                        df.name = name.to_string();
                        Some(df)
                    },
                    FieldOrRef::Ref { .. } => None,
                }
            })
            .collect();

        DeclStruct {
            name: s.meta.name.to_string(),
            fields,
            meta: DeclMeta::new(s.meta.version.get() as u32),
            comments: doc_to_comment(&s.meta.description),
        }
    }
}

impl From<&Enum> for DeclEnumDef {
    fn from(e: &Enum) -> Self {
        let variants: Vec<_> = e.variants.iter().collect();
        let first_ty = variants.first().map(|(_, v)| &v.value);

        let enum_def = match first_ty {
            Some(StrOrInt::Int(_)) | None => {
                DeclEnum::Int(
                    e.variants
                        .iter()
                        .map(|(name, v)| {
                            DeclIntVariant {
                                name: name.to_string(),
                                value: match &v.value {
                                    StrOrInt::Int(i) => *i as u32,
                                    _ => 0,
                                },
                                comments: doc_to_comment(&v.meta.description),
                            }
                        })
                        .collect(),
                )
            },
            Some(StrOrInt::String(_)) => {
                DeclEnum::String(
                    e.variants
                        .iter()
                        .map(|(name, v)| {
                            DeclStringVariant {
                                name: name.to_string(),
                                value: match &v.value {
                                    StrOrInt::String(s) => s.clone(),
                                    _ => name.to_string(),
                                },
                                comments: doc_to_comment(&v.meta.description),
                            }
                        })
                        .collect(),
                )
            },
        };

        DeclEnumDef {
            name: e.meta.name.to_string(),
            enum_def,
            meta: DeclMeta::new(e.meta.version.get() as u32),
            comments: doc_to_comment(&e.meta.description),
        }
    }
}

impl From<&OneOfVariant> for DeclOneOfVariant {
    fn from(v: &OneOfVariant) -> Self {
        DeclOneOfVariant {
            name: v.name.to_string(),
            ty: (&v.ty).into(),
            comments: doc_to_comment(&v.description),
        }
    }
}

impl From<&OneOf> for DeclOneOf {
    fn from(o: &OneOf) -> Self {
        DeclOneOf {
            name: o.meta.name.to_string(),
            variants: o
                .variants
                .iter()
                .map(|(_, v)| v.into())
                .collect(),
            meta: DeclMeta::new(o.meta.version.get() as u32),
            comments: doc_to_comment(&o.meta.description),
        }
    }
}

impl From<&ErrorTy> for DeclError {
    fn from(e: &ErrorTy) -> Self {
        DeclError {
            name: e.meta.name.to_string(),
            variants: e
                .variants
                .iter()
                .map(|(_, v)| v.into())
                .collect(),
            meta: DeclMeta::new(e.meta.version.get() as u32),
            comments: doc_to_comment(&e.meta.description),
        }
    }
}

impl From<&Operation> for DeclOperation {
    fn from(op: &Operation) -> Self {
        let args: Vec<DeclArg> = op
            .inputs
            .iter()
            .filter_map(|(name, field_or_ref)| {
                match field_or_ref {
                    FieldOrRef::Value(f) => {
                        Some(DeclArg {
                            name: name.to_string(),
                            ty: (&f.ty).into(),
                            default_value: None,
                            comments: doc_to_comment(&f.meta.description),
                        })
                    },
                    FieldOrRef::Ref { .. } => None,
                }
            })
            .collect();

        let return_type = op
            .outputs
            .iter()
            .next()
            .and_then(|(_, field_or_ref)| {
                match field_or_ref {
                    FieldOrRef::Value(f) => Some((&f.ty).into()),
                    FieldOrRef::Ref { .. } => None,
                }
            })
            .unwrap_or(DeclType::Builtin { ty: Builtin::Never });

        DeclOperation {
            name: op.meta.name.to_string(),
            args,
            return_type,
            meta: DeclMeta::new(op.meta.version.get() as u32),
            comments: doc_to_comment(&op.meta.description),
        }
    }
}

impl From<&Definitions> for TypeDefinition {
    fn from(def: &Definitions) -> Self {
        match def {
            Definitions::StructV1(s) => TypeDefinition::Struct(s.into()),
            Definitions::EnumV1(e) => TypeDefinition::Enum(e.into()),
            Definitions::OneOfV1(o) => TypeDefinition::OneOf(o.into()),
            Definitions::ErrorV1(e) => TypeDefinition::Error(e.into()),
            Definitions::OperationV1(op) => TypeDefinition::Operation(op.into()),
            Definitions::FieldV1(_) | Definitions::NamespaceV1(_) => {
                panic!("Field and Namespace are not TypeDefinitions")
            },
        }
    }
}

impl From<&Namespace> for DeclNamespace {
    fn from(ns: &Namespace) -> Self {
        let mut types = Vec::new();

        for s in ns.defs.values() {
            types.push(TypeDefinition::Struct(s.into()));
        }

        for e in ns.enums.values() {
            types.push(TypeDefinition::Enum(e.into()));
        }

        for o in ns.one_ofs.values() {
            types.push(TypeDefinition::OneOf(o.into()));
        }

        for e in ns.errors.values() {
            types.push(TypeDefinition::Error(e.into()));
        }

        for op in ns.ops.values() {
            types.push(TypeDefinition::Operation(op.into()));
        }

        DeclNamespace {
            name: ns.name.to_string(),
            version: Some(ns.version.get() as u32),
            error: None,
            types,
            namespaces: Default::default(),
            comments: DeclComment::default(),
        }
    }
}

impl Version {
    pub fn as_u32(&self) -> u32 {
        self.get() as u32
    }
}
