use std::path::PathBuf;

use crate::{
    ast::{
        anonymous::AnonymousStruct,
        array::Array,
        items::{OneOfDef, OperationDef, StructDef},
        one_of::AnonymousOneOf,
        ty::Type,
        union::{IdentOrUnion, Union},
        variadic::Variant,
    },
    ctx::common::{FromNamedSource, NamespaceChild},
    defs::Spanned,
};

use super::helpers::{NameContext, build_struct_def_from_anonymous};

pub(super) fn from_child(
    name_gen: &mut NameContext,
    child: &FromNamedSource<NamespaceChild>,
) -> crate::Result<Vec<FromNamedSource<StructDef>>> {
    match &child.value {
        NamespaceChild::Struct(struct_def) => {
            from_struct_fields(name_gen, struct_def, &child.source)
        },
        NamespaceChild::Type(type_def) => {
            from_type(name_gen, &type_def.def.value.ty.value, &child.source)
        },
        NamespaceChild::Operation(op_def) => from_operation(name_gen, op_def, &child.source),
        NamespaceChild::OneOf(oneof_def) => from_oneof(name_gen, oneof_def, &child.source),
        NamespaceChild::Namespace(ns_ctx) => {
            let mut extracted = Vec::new();
            for (nested_name, nested_child) in &ns_ctx.children {
                name_gen.push(nested_name.name.borrow_string().clone());
                extracted.extend(from_child(name_gen, nested_child)?);
                name_gen.pop();
            }
            Ok(extracted)
        },
        _ => Ok(Vec::new()),
    }
}

fn from_struct_fields(
    name_gen: &mut NameContext,
    struct_def: &StructDef,
    source: &PathBuf,
) -> crate::Result<Vec<FromNamedSource<StructDef>>> {
    // Anonymous structs from struct fields are extracted in extract_anonymous_structs()
    // during register_types_recursive(), BEFORE type resolution.
    // This ensures they're in the type registry for declaration conversion.
    // Here we only need to traverse into nested types that aren't direct anonymous structs.
    let mut extracted = Vec::new();

    for field in &struct_def.def.value.args.values {
        name_gen.push(field.value.name.borrow_string().clone());
        // Skip Type::Struct - these are extracted early in extract_anonymous_structs
        // but still recurse into other types (arrays, oneofs, etc)
        match &field.value.typ {
            Type::Struct { .. } => {
                // Already extracted in extract_anonymous_structs - skip
            },
            _ => {
                extracted.extend(from_type(name_gen, &field.value.typ, source)?);
            },
        }
        name_gen.pop();
    }

    Ok(extracted)
}

fn from_operation(
    name_gen: &mut NameContext,
    op_def: &OperationDef,
    source: &PathBuf,
) -> crate::Result<Vec<FromNamedSource<StructDef>>> {
    let mut extracted = Vec::new();

    name_gen.push(op_def.def.value.name.borrow_string().clone());

    if let Some(args) = op_def
        .def
        .value
        .args
        .as_ref()
        .map(|ok| &ok.value.values)
    {
        for arg in args {
            name_gen.push(arg.value.name.borrow_string().clone());
            extracted.extend(from_type(name_gen, &arg.value.typ, source)?);
            name_gen.pop();
        }
    }

    name_gen.push("Result");
    extracted.extend(from_type(
        name_gen,
        &op_def.def.value.return_type.value,
        source,
    )?);
    name_gen.pop(); // result
    name_gen.pop(); // operation

    Ok(extracted)
}

fn from_oneof(
    name_gen: &mut NameContext,
    oneof_def: &OneOfDef,
    source: &PathBuf,
) -> crate::Result<Vec<FromNamedSource<StructDef>>> {
    let mut extracted = Vec::new();

    for (idx, variant) in oneof_def
        .def
        .value
        .variants
        .values
        .iter()
        .enumerate()
    {
        name_gen.push(format!("Variant{}", idx));
        match &variant.value.value {
            Variant::Tuple { inner, .. } => {
                extracted.extend(from_type(name_gen, inner, source)?);
            },
            Variant::LocalStruct { inner, .. } => {
                // LocalStruct variants are extracted in register_types_recursive with
                // proper {ParentName}{VariantName} naming per RFC-0008.
                // Here we only extract nested anonymous structs from the fields.
                for field in &inner.value.fields.value.values {
                    name_gen.push(field.value.name.borrow_string().clone());
                    extracted.extend(from_type(name_gen, &field.value.typ, source)?);
                    name_gen.pop();
                }
            },
        }
        name_gen.pop();
    }

    Ok(extracted)
}

fn from_type(
    name_gen: &mut NameContext,
    typ: &Type,
    source: &PathBuf,
) -> crate::Result<Vec<FromNamedSource<StructDef>>> {
    let mut extracted = Vec::new();

    match typ {
        Type::Struct { ty } => {
            extracted.extend(anonymous_struct(name_gen, ty, source)?);
        },
        Type::Array { ty } => {
            extracted.extend(from_array(name_gen, &ty.value, source)?);
        },
        Type::Union { ty } => {
            extracted.extend(from_union(&ty.value, source)?);
        },
        Type::OneOf { ty } => {
            extracted.extend(from_anonymous_oneof(name_gen, &ty.value, source)?);
        },
        Type::Paren { ty, .. } => {
            extracted.extend(from_type(name_gen, &ty.value, source)?);
        },
        Type::Result { ty, .. } => {
            extracted.extend(from_type(name_gen, &ty.value, source)?);
        },
        _ => {},
    }

    Ok(extracted)
}

fn anonymous_struct(
    name_gen: &mut NameContext,
    anonymous: &Spanned<AnonymousStruct>,
    source: &PathBuf,
) -> crate::Result<Vec<FromNamedSource<StructDef>>> {
    let mut extracted = Vec::new();

    for field in &anonymous.value.fields.value.values {
        name_gen.push(field.value.name.borrow_string().clone());
        extracted.extend(from_type(name_gen, &field.value.typ, source)?);
        name_gen.pop();
    }

    let generated_name = name_gen.generate_name();
    let struct_def =
        build_struct_def_from_anonymous(generated_name, anonymous.value.clone(), source.clone());

    extracted.push(struct_def);

    Ok(extracted)
}

fn from_array(
    name_gen: &mut NameContext,
    array: &Array,
    source: &PathBuf,
) -> crate::Result<Vec<FromNamedSource<StructDef>>> {
    match array {
        Array::Sized { ty, .. } | Array::Unsized { ty, .. } => {
            from_type(name_gen, &ty.value, source)
        },
    }
}

#[allow(clippy::only_used_in_recursion)]
fn from_union(
    union: &Union,
    source: &PathBuf,
) -> crate::Result<Vec<FromNamedSource<StructDef>>> {
    let mut extracted = Vec::new();

    for operand in &union.types.values {
        match &operand.value.value {
            IdentOrUnion::Union { inner, .. } => {
                extracted.extend(from_union(&inner.value, source)?);
            },
            IdentOrUnion::Ident(_) => {},
        }
    }

    Ok(extracted)
}

fn from_anonymous_oneof(
    name_gen: &mut NameContext,
    oneof: &AnonymousOneOf,
    source: &PathBuf,
) -> crate::Result<Vec<FromNamedSource<StructDef>>> {
    let mut extracted = Vec::new();

    for (idx, variant) in oneof
        .variants
        .value
        .values
        .iter()
        .enumerate()
    {
        name_gen.push(format!("Variant{}", idx));
        extracted.extend(from_type(name_gen, &variant.value.value, source)?);
        name_gen.pop();
    }

    Ok(extracted)
}
