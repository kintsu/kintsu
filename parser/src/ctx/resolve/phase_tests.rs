use std::path::PathBuf;

use crate::{
    ast::{
        items::Item,
        namespace::Namespace,
        ty::{Builtin, Type},
    },
    ctx::{
        NamespaceCtx, RefContext, common::WithSource, registry::TypeRegistry, resolve::TypeResolver,
    },
    tokens::ToTokens,
    tst::*,
};

#[tokio::test]
#[ignore = "struct field anonymous structs are extracted in extract_anonymous_structs, not TypeResolver"]
async fn test_simple_anonymous_struct() {
    // struct User { profile: { name: str, age: i32 } };
    let src = r#"struct User {
        profile: {
            name: str,
            age: i32
        }
    };"#;

    let resolver = resolver_with(vec![struct_def("User", src)]).await;

    let result = resolver
        .resolve()
        .await
        .expect("resolution failed");

    // verify: should extract UserProfile struct
    assert_eq!(
        result.anonymous_structs.len(),
        1,
        "Expected 1 anonymous struct to be extracted"
    );

    let extracted = &result.anonymous_structs[0].value;
    assert_eq!(
        extracted
            .value
            .def
            .value
            .name
            .borrow_string(),
        "UserProfile",
        "Generated name should be UserProfile"
    );

    // verify fields: name and age
    assert_eq!(
        extracted.value.def.value.args.values.len(),
        2,
        "UserProfile should have 2 fields"
    );

    let field_names: Vec<String> = extracted
        .value
        .def
        .value
        .args
        .values
        .iter()
        .map(|f| f.value.name.borrow_string().clone())
        .collect();

    assert!(field_names.contains(&"name".to_string()));
    assert!(field_names.contains(&"age".to_string()));
}

#[tokio::test]
#[ignore] // todo: support empty {} syntax
async fn test_empty_anonymous_struct() {
    // struct Container { placeholder: {} };
    let src = r#"struct Container {
        placeholder: {}
    };"#;

    let resolver = resolver_with(vec![struct_def("Container", src)]).await;

    let result = resolver
        .resolve()
        .await
        .expect("resolution failed");

    // should extract ContainerPlaceholder with 0 fields
    assert_eq!(
        result.anonymous_structs.len(),
        1,
        "Expected 1 anonymous struct"
    );

    let extracted = &result.anonymous_structs[0].value;
    assert_eq!(
        extracted
            .value
            .def
            .value
            .name
            .borrow_string(),
        "ContainerPlaceholder"
    );

    assert_eq!(
        extracted.value.def.value.args.values.len(),
        0,
        "Empty anonymous struct should have 0 fields"
    );
}

#[tokio::test]
#[ignore = "struct field anonymous structs are extracted in extract_anonymous_structs, not TypeResolver"]
async fn test_nested_anonymous_structs() {
    // struct User { metadata: { info: { created: str } } };
    let src = r#"struct User {
        metadata: {
            info: {
                created: str
            }
        }
    };"#;

    let resolver = resolver_with(vec![struct_def("User", src)]).await;

    let result = resolver
        .resolve()
        .await
        .expect("resolution failed");

    // should extract UserMetadataInfo and UserMetadata
    assert_eq!(
        result.anonymous_structs.len(),
        2,
        "Expected 2 anonymous structs (nested)"
    );

    let names: Vec<String> = result
        .anonymous_structs
        .iter()
        .map(|s| {
            s.value
                .value
                .def
                .value
                .name
                .borrow_string()
                .clone()
        })
        .collect();

    assert!(
        names.contains(&"UserMetadata".to_string()),
        "Should extract UserMetadata"
    );
    assert!(
        names.contains(&"UserMetadataInfo".to_string()),
        "Should extract UserMetadataInfo"
    );
}

#[tokio::test]
#[ignore = "struct field anonymous structs are extracted in extract_anonymous_structs, not TypeResolver"]
async fn test_deep_nesting() {
    // 4 levels deep
    let src = r#"struct Root {
        level1: {
            level2: {
                level3: {
                    value: str
                }
            }
        }
    };"#;

    let resolver = resolver_with(vec![struct_def("Root", src)]).await;

    let result = resolver
        .resolve()
        .await
        .expect("resolution failed");

    // should extract 3 structs: RootLevel1, RootLevel1Level2, RootLevel1Level2Level3
    assert_eq!(
        result.anonymous_structs.len(),
        3,
        "Expected 3 anonymous structs for 4-level nesting"
    );

    let names: Vec<String> = result
        .anonymous_structs
        .iter()
        .map(|s| {
            s.value
                .value
                .def
                .value
                .name
                .borrow_string()
                .clone()
        })
        .collect();

    assert!(names.contains(&"RootLevel1".to_string()));
    assert!(names.contains(&"RootLevel1Level2".to_string()));
    assert!(names.contains(&"RootLevel1Level2Level3".to_string()));
}

#[tokio::test]
#[ignore = "struct field anonymous structs are extracted in extract_anonymous_structs, not TypeResolver"]
async fn test_multiple_anonymous_fields() {
    // struct Data { config: { x: i32 }, state: { y: i32 } };
    let src = r#"struct Data {
        config: {
            x: i32
        },
        state: {
            y: i32
        }
    };"#;

    let resolver = resolver_with(vec![struct_def("Data", src)]).await;

    let result = resolver
        .resolve()
        .await
        .expect("resolution failed");

    // should extract DataConfig and DataState
    assert_eq!(
        result.anonymous_structs.len(),
        2,
        "Expected 2 anonymous structs (multiple fields)"
    );

    let names: Vec<String> = result
        .anonymous_structs
        .iter()
        .map(|s| {
            s.value
                .value
                .def
                .value
                .name
                .borrow_string()
                .clone()
        })
        .collect();

    assert!(names.contains(&"DataConfig".to_string()));
    assert!(names.contains(&"DataState".to_string()));
}

#[tokio::test]
async fn test_simple_field_union() {
    // struct Request { auth: User & Permissions };
    let src = r#"struct Request {
        auth: User & Permissions
    };"#;

    let resolver = resolver_with(vec![
        struct_def("User", "struct User { id: i64 };"),
        struct_def(
            "Permissions",
            "struct Permissions { resource: i64, level: usize };",
        ),
        struct_def("Request", src),
    ])
    .await;

    let result = resolver
        .resolve()
        .await
        .expect("resolution failed");

    // Should identify 1 union
    assert_eq!(
        result.identified_unions.len(),
        1,
        "Expected 1 union to be identified"
    );

    let union_record = &result.identified_unions[0].0.value;

    // Check context stack: should be ["Request", "auth"]
    assert_eq!(
        union_record.context_stack.len(),
        2,
        "Context stack should have 2 elements"
    );
    assert_eq!(union_record.context_stack[0], "Request");
    assert_eq!(union_record.context_stack[1], "auth");

    // Generated name should be "RequestAuth"
    assert_eq!(union_record.generate_name(), "RequestAuth");

    // Should not be in oneof
    assert!(!union_record.in_oneof);
    assert_eq!(union_record.variant_index, None);
}

#[tokio::test]
async fn test_nested_union() {
    // struct Data { combined: A & (B & C) };
    let src = r#"struct Data {
        combined: A & (B & C)
    };"#;

    let resolver = resolver_with(vec![
        struct_def("A", "struct A { a: i32 };"),
        struct_def("B", "struct B { b: i32 };"),
        struct_def("C", "struct C { c: i32 };"),
        struct_def("Data", src),
    ])
    .await;
    let result = resolver
        .resolve()
        .await
        .expect("resolution failed");

    // Should identify the outer union only
    // (nested parenthesized unions are part of the same union expression)
    assert_eq!(
        result.identified_unions.len(),
        1,
        "Expected 1 union (outer union includes nested)"
    );

    let union_record = &result.identified_unions[0].0.value;
    assert_eq!(union_record.generate_name(), "DataCombined");
}

#[tokio::test]
async fn test_multiple_unions_in_struct() {
    // struct Config { auth: User & Permissions, data: Base & Extensions };
    let src = r#"struct Config {
        auth: User & Permissions,
        data: Base & Extensions
    };"#;

    let resolver = resolver_with(vec![
        struct_def("User", "struct User { id: i64 };"),
        struct_def("Permissions", "struct Permissions { level: i32 };"),
        struct_def("Base", "struct Base { foo: str };"),
        struct_def("Extensions", "struct Extensions { created_at: datetime };"),
        struct_def("Config", src),
    ])
    .await;

    let result = resolver
        .resolve()
        .await
        .expect("resolution failed");

    // Should identify 2 unions
    assert_eq!(
        result.identified_unions.len(),
        2,
        "Expected 2 unions to be identified"
    );

    let names: Vec<String> = result
        .identified_unions
        .iter()
        .map(|u| u.0.value.generate_name())
        .collect();

    assert!(names.contains(&"ConfigAuth".to_string()));
    assert!(names.contains(&"ConfigData".to_string()));
}

#[tokio::test]
async fn test_union_in_array() {
    crate::tst::logging();

    let resolver = resolver_with(vec![
        struct_def("Base", "struct Base { foo: i32 };"),
        struct_def("Extensions", "struct Extensions { created_at: datetime };"),
        struct_def(
            "Container",
            "struct Container { items: (Base & Extensions)[] };",
        ),
    ])
    .await;

    let result = resolver
        .resolve()
        .await
        .expect("resolution failed");

    // Should identify union inside array
    assert_eq!(
        result.identified_unions.len(),
        1,
        "Expected 1 union (inside array)"
    );

    let union_record = &result.identified_unions[0].0.value;
    assert_eq!(union_record.generate_name(), "ContainerItems");
}

#[tokio::test]
async fn test_union_in_oneof_variant() {
    // type Response = oneof (A & B) | (C & D);
    let src = r#"type Response = oneof (A & B) | (C & D);"#;

    let resolver = resolver_with(vec![
        struct_def("A", "struct A { a: i32 };"),
        struct_def("B", "struct B { b: i32 };"),
        struct_def("C", "struct C { c: i32 };"),
        struct_def("D", "struct D { d: i32 };"),
        type_alias("Response", src),
    ])
    .await;

    let result = resolver
        .resolve()
        .await
        .expect("resolution failed");

    // Should identify 2 unions (one per variant)
    assert_eq!(
        result.identified_unions.len(),
        2,
        "Expected 2 unions (one per oneof variant)"
    );

    let names: Vec<String> = result
        .identified_unions
        .iter()
        .map(|u| u.0.value.generate_name())
        .collect();

    // Should have numeric suffixes
    assert!(names.contains(&"Response1".to_string()));
    assert!(names.contains(&"Response2".to_string()));

    // Check in_oneof flags
    for union_record in &result.identified_unions {
        assert!(
            union_record.0.value.in_oneof,
            "Should be marked as in_oneof"
        );
        assert!(
            union_record.0.value.variant_index.is_some(),
            "Should have variant index"
        );
    }
}

#[tokio::test]
async fn test_simple_alias_chain() {
    crate::tst::logging();

    let resolver = resolver_with(vec![
        type_alias("UserId", "type UserId = i64;"),
        type_alias("PrimaryId", "type PrimaryId = UserId;"),
        type_alias("RecordId", "type RecordId = PrimaryId;"),
    ])
    .await;

    let result = resolver
        .resolve()
        .await
        .expect("resolution should succeed");

    assert_eq!(result.resolved_aliases.len(), 3);
    assert!(matches!(
        result
            .resolved_aliases
            .get("RecordId")
            .map(|r| &r.value),
        Some(Type::Builtin { .. })
    ));
}

#[tokio::test]
async fn test_circular_alias_detection() {
    crate::tst::logging();

    let resolver = resolver_with_checked(vec![
        type_alias("A", "type A = B;"),
        type_alias("B", "type B = C;"),
        type_alias("C", "type C = A;"),
    ])
    .await;

    assert!(resolver.is_err(), "Should detect circular type alias");
}

#[tokio::test]
async fn test_complex_type_alias_array() {
    crate::tst::logging();

    let resolver = resolver_with(vec![
        struct_def("User", "struct User { id: i64, name: str };"),
        type_alias("UserList", "type UserList = User[];"),
        type_alias("AdminList", "type AdminList = UserList;"),
    ])
    .await;

    let result = resolver
        .resolve()
        .await
        .expect("resolution should succeed");

    assert_eq!(result.resolved_aliases.len(), 2);
    assert!(matches!(
        result
            .resolved_aliases
            .get("AdminList")
            .map(|r| &r.value),
        Some(Type::Array { .. })
    ));
}

#[tokio::test]
async fn test_union_type_alias() {
    crate::tst::logging();

    let resolver = resolver_with(vec![
        struct_def("Base", "struct Base { id: i64 };"),
        struct_def("Extensions", "struct Extensions { extra: str };"),
        type_alias("Combined", "type Combined = Base & Extensions;"),
        type_alias("Enhanced", "type Enhanced = Combined;"),
    ])
    .await;

    let result = resolver
        .resolve()
        .await
        .expect("resolution should succeed");

    assert_eq!(result.resolved_aliases.len(), 2);
    assert!(matches!(
        result
            .resolved_aliases
            .get("Enhanced")
            .map(|r| &r.value),
        Some(Type::Union { .. })
    ));
}

#[tokio::test]
async fn test_alias_to_non_alias() {
    crate::tst::logging();

    let resolver = resolver_with(vec![
        struct_def("User", "struct User { id: i64 };"),
        type_alias("UserId", "type UserId = User;"),
    ])
    .await;
    let result = resolver
        .resolve()
        .await
        .expect("resolution should succeed");

    assert_eq!(result.resolved_aliases.len(), 1);
    match &result
        .resolved_aliases
        .get("UserId")
        .unwrap()
        .value
    {
        Type::Ident { to } => {
            match to {
                crate::ast::ty::PathOrIdent::Ident(ident) => {
                    assert_eq!(ident.borrow_string(), "User");
                },
                _ => panic!("Expected ident"),
            }
        },
        _ => panic!("Expected ident type"),
    }
}

#[tokio::test]
async fn test_valid_struct_union() {
    let resolver = resolver_with(vec![
        struct_def("Base", "struct Base { id: i64 };"),
        struct_def("Extensions", "struct Extensions { extra: str };"),
        type_alias("Combined", "type Combined = Base & Extensions;"),
    ])
    .await;

    let result = resolver.resolve().await;

    assert!(result.is_ok(), "Valid struct union should pass validation");
}

#[tokio::test]
async fn test_invalid_builtin_union() {
    let resolver = resolver_with(vec![
        type_alias("A", "type Invalid = i32;"),
        type_alias("B", "type Invalid = u32;"),
        type_alias("Invalid", "type Invalid = A & B;"),
    ])
    .await;

    let result = resolver.resolve().await;

    assert!(result.is_err(), "Builtin union should fail validation");
    match result {
        Err(crate::Error::Compiler(crate::CompilerError::Union(
            crate::UnionError::UnionOperandNotStruct { found_type, .. },
        ))) => {
            assert_eq!(found_type, "i32");
        },
        other => {
            panic!(
                "Expected UnionOperandNotStruct error, got {}",
                match other {
                    Err(e) => format!("{:?}", e),
                    Ok(..) => "a valid namespace".into(),
                }
            )
        },
    }
}

#[tokio::test]
async fn test_invalid_mixed_union() {
    let resolver = resolver_with(vec![
        struct_def("Data", "struct Data { value: i32 };"),
        type_alias("A", "type A = i64;"),
        type_alias("Invalid", "type Invalid = Data & A;"),
    ])
    .await;

    let result = resolver.resolve().await;

    assert!(
        result.is_err(),
        "Mixed struct/builtin union should fail validation"
    );
    match result {
        Err(crate::Error::Compiler(crate::CompilerError::Union(
            crate::UnionError::UnionOperandNotStruct { found_type, .. },
        ))) => {
            assert_eq!(found_type, "i64");
        },
        other => {
            panic!(
                "Expected UnionOperandNotStruct error, got {}",
                match other {
                    Err(e) => format!("{:?}", e),
                    Ok(..) => "a valid namespace".into(),
                }
            )
        },
    }
}

#[tokio::test]
async fn test_invalid_enum_union() {
    let resolver = resolver_with(vec![
        struct_def("Record", "struct Record { id: i64 };"),
        enum_def("Status", "enum Status { Active, Inactive };"),
        type_alias("Invalid", "type Invalid = Record & Status;"),
    ])
    .await;

    let result = resolver.resolve().await;

    assert!(result.is_err(), "Enum union should fail validation");
    match result {
        Err(crate::Error::Compiler(crate::CompilerError::Union(
            crate::UnionError::UnionOperandNotStruct { found_type, .. },
        ))) => {
            assert_eq!(found_type, "enum");
        },
        other => {
            panic!(
                "Expected UnionOperandNotStruct error, got {}",
                match other {
                    Err(e) => format!("{:?}", e),
                    Ok(..) => "a valid namespace".into(),
                }
            )
        },
    }
}

#[tokio::test]
async fn test_union_with_resolved_alias() {
    let resolver = resolver_with(vec![
        struct_def("Base", "struct Base { id: i64 };"),
        struct_def("Extra", "struct Extra { data: str };"),
        type_alias("AliasedExtra", "type AliasedExtra = Extra;"),
        type_alias("Combined", "type Combined = Base & AliasedExtra;"),
    ])
    .await;

    let result = resolver.resolve().await;

    assert!(
        result.is_ok(),
        "Union with struct alias should pass validation"
    );
}

#[tokio::test]
async fn test_nested_union_validation() {
    let resolver = resolver_with(vec![
        struct_def("A", "struct A { a: i32 };"),
        struct_def("B", "struct B { b: i32 };"),
        struct_def("C", "struct C { c: i32 };"),
        type_alias("Complex", "type Complex = A & (B & C);"),
    ])
    .await;

    let result = resolver.resolve().await;

    assert!(result.is_ok(), "Nested struct union should pass validation");
}

#[tokio::test]
async fn test_simple_union_merge() {
    let resolver = resolver_with(vec![
        struct_def("Base", "struct Base { id: i64 };"),
        struct_def("Extra", "struct Extra { data: str };"),
        type_alias("Combined", "type Combined = Base & Extra;"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Resolution should succeed");

    // Verify merged struct was created
    assert_eq!(
        resolution.union_structs.len(),
        1,
        "Should create one merged struct"
    );

    // Verify the merged struct has both fields
    let merged = &resolution.union_structs[0];
    let fields: Vec<_> = merged
        .def
        .value
        .args
        .values
        .iter()
        .map(|arg| arg.value.name.value.borrow_string().clone())
        .collect();

    assert!(
        fields.contains(&"id".to_string()),
        "Should have 'id' field from Base"
    );
    assert!(
        fields.contains(&"data".to_string()),
        "Should have 'data' field from Extra"
    );
    assert_eq!(fields.len(), 2, "Should have exactly 2 fields");
}

#[tokio::test]
async fn test_field_conflict_leftmost_wins() {
    let resolver = resolver_with(vec![
        struct_def("Identity", "struct Identity { version: i32, id: i64 };"),
        struct_def(
            "Timestamps",
            "struct Timestamps { version: i64, created_at: datetime };",
        ),
        type_alias("Record", "type Record = Identity & Timestamps;"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Resolution should succeed");

    // Verify merged struct
    assert_eq!(resolution.union_structs.len(), 1);

    let merged = &resolution.union_structs[0];
    let version_field = merged
        .def
        .value
        .args
        .values
        .iter()
        .find(|arg| arg.value.name.value.borrow_string() == "version")
        .expect("Should have version field");

    // Verify the version field type is i32 (from Identity, leftmost)
    if let Type::Builtin { ty } = &version_field.value.typ {
        match &ty.value {
            Builtin::I32(..) => {},
            ty => panic!("expected resolved i32 and found {}", ty.display()),
        }
    } else {
        panic!("version field should be an Ident type");
    }

    // Verify all unique fields are present
    let field_names: Vec<_> = merged
        .def
        .value
        .args
        .values
        .iter()
        .map(|arg| arg.value.name.value.borrow_string().clone())
        .collect();

    assert!(field_names.contains(&"id".to_string()));
    assert!(field_names.contains(&"version".to_string()));
    assert!(field_names.contains(&"created_at".to_string()));
    assert_eq!(field_names.len(), 3, "Should have 3 unique fields");
}

#[tokio::test]
async fn test_nested_union_merge() {
    let resolver = resolver_with(vec![
        struct_def("A", "struct A { a: i32 };"),
        struct_def("B", "struct B { b: i32 };"),
        struct_def("C", "struct C { c: i32 };"),
        type_alias("Complex", "type Complex = A & (B & C);"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Resolution should succeed");

    // Verify all fields merged
    assert_eq!(resolution.union_structs.len(), 1);

    let merged = &resolution.union_structs[0];
    let field_names: Vec<_> = merged
        .def
        .value
        .args
        .values
        .iter()
        .map(|arg| arg.value.name.value.borrow_string().clone())
        .collect();

    assert!(field_names.contains(&"a".to_string()));
    assert!(field_names.contains(&"b".to_string()));
    assert!(field_names.contains(&"c".to_string()));
    assert_eq!(
        field_names.len(),
        3,
        "Should merge all fields from nested unions"
    );
}

#[tokio::test]
async fn test_anonymous_operand_merge() {
    let resolver = resolver_with(vec![
        struct_def("Base", "struct Base { id: i64 };"),
        type_alias("Extended", "type Extended = Base & { extra: str };"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Resolution should succeed");

    // Verify fields merged
    assert_eq!(resolution.union_structs.len(), 1);

    let merged = &resolution.union_structs[0];
    let field_names: Vec<_> = merged
        .def
        .value
        .args
        .values
        .iter()
        .map(|arg| arg.value.name.value.borrow_string().clone())
        .collect();

    assert!(field_names.contains(&"id".to_string()));
    assert!(field_names.contains(&"extra".to_string()));
    assert_eq!(
        field_names.len(),
        2,
        "Should merge Base and anonymous struct fields"
    );
}

#[tokio::test]
async fn test_multiple_anonymous_operands() {
    let resolver = resolver_with(vec![type_alias(
        "Combined",
        "type Combined = { a: i32 } & { b: str };",
    )])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Resolution should succeed");

    assert_eq!(resolution.union_structs.len(), 1);

    let merged = &resolution.union_structs[0];
    let field_names: Vec<_> = merged
        .def
        .value
        .args
        .values
        .iter()
        .map(|arg| arg.value.name.value.borrow_string().clone())
        .collect();

    assert!(field_names.contains(&"a".to_string()));
    assert!(field_names.contains(&"b".to_string()));
    assert_eq!(field_names.len(), 2);
}

#[tokio::test]
async fn test_mixed_ref_and_anonymous() {
    let resolver = resolver_with(vec![
        struct_def("A", "struct A { a: i32 };"),
        struct_def("B", "struct B { b: str };"),
        type_alias("Mixed", "type Mixed = A & { x: i64 } & B;"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Resolution should succeed");

    assert_eq!(resolution.union_structs.len(), 1);

    let merged = &resolution.union_structs[0];
    let field_names: Vec<_> = merged
        .def
        .value
        .args
        .values
        .iter()
        .map(|arg| arg.value.name.value.borrow_string().clone())
        .collect();

    assert!(field_names.contains(&"a".to_string()));
    assert!(field_names.contains(&"x".to_string()));
    assert!(field_names.contains(&"b".to_string()));
    assert_eq!(field_names.len(), 3);
}

#[tokio::test]
async fn test_version_item_override() {
    let resolver = resolver_with(vec![
        struct_def("User", "#[version(5)] struct User { id: i64 };"),
        struct_def("Post", "struct Post { id: i64 };"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Resolution should succeed");

    // User should have version 5 from item meta
    assert_eq!(
        resolution
            .versions
            .get("User")
            .map(|v| v.value),
        Some(5)
    );

    // Post should have default version 1
    assert_eq!(
        resolution
            .versions
            .get("Post")
            .map(|v| v.value),
        Some(1)
    );
}

#[tokio::test]
async fn test_version_namespace_inheritance() {
    crate::tst::logging();

    // Create namespace with inner version
    let ctx = RefContext::new("test_package".to_string(), vec!["test".to_string()]);
    let ns_src = "#![version(3)] namespace test;";
    let ns_def: Item<Namespace> = crate::tst::basic_smoke(ns_src).unwrap();

    let mut ns = NamespaceCtx {
        ctx: ctx.clone(),
        tag: None,
        sources: Default::default(),
        comments: vec![],
        error: None,
        version: {
            // Extract version from namespace meta
            ns_def
                .meta()
                .iter()
                .find_map(|meta_spanned| {
                    meta_spanned
                        .value
                        .meta
                        .iter()
                        .find_map(|item| {
                            if let crate::ast::meta::ItemMetaItem::Version(v) = item {
                                Some(
                                    v.clone()
                                        .with_source(PathBuf::from("test.ks")),
                                )
                            } else {
                                None
                            }
                        })
                })
        },
        namespace: ns_def.with_source("test.ks".into()),
        imports: Vec::new(),
        children: Default::default(),
        registry: TypeRegistry::new(),
        resolved_errors: Default::default(),
        resolved_versions: Default::default(),
        resolved_aliases: Default::default(),
    };

    // Add struct without explicit version - should inherit from namespace
    add_struct_def(&mut ns, "User", "struct User { id: i64 };").await;

    // Add struct with explicit version - should override
    add_struct_def(&mut ns, "Admin", "#[version(7)] struct Admin { id: i64 };").await;

    let ns = crate::tst::register_namespace_types(ns)
        .await
        .expect("Failed to register types");
    let resolver = TypeResolver::new(ns);
    let resolution = resolver
        .resolve()
        .await
        .expect("Resolution should succeed");

    // User should inherit namespace version 3
    assert_eq!(
        resolution
            .versions
            .get("User")
            .map(|v| v.value),
        Some(3)
    );

    // Admin should use explicit version 7
    assert_eq!(
        resolution
            .versions
            .get("Admin")
            .map(|v| v.value),
        Some(7)
    );
}

#[tokio::test]
async fn test_version_enum_and_oneof() {
    let resolver = resolver_with(vec![
        enum_def(
            "Status",
            "#[version(2)] enum Status { Active = 1, Inactive = 2 };",
        ),
        oneof_def(
            "Choice",
            "#[version(4)] oneof Choice { A{ id: i64 }, B{ id: i64 } };",
        ),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Resolution should succeed");

    assert_eq!(
        resolution
            .versions
            .get("Status")
            .map(|v| v.value),
        Some(2)
    );
    assert_eq!(
        resolution
            .versions
            .get("Choice")
            .map(|v| v.value),
        Some(4)
    );
}

#[tokio::test]
async fn test_error_operation_explicit() {
    let resolver = resolver_with(vec![
        error_def("UserError", "error UserError { UserExists { id: i64 } };"),
        struct_def("User", "struct User { id: i64 };"),
        operation_def(
            "create_user",
            "#[err(UserError)] operation create_user() -> User!;",
        ),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Resolution should succeed");

    assert_eq!(
        resolution
            .errors
            .get("create_user")
            .map(|e| e.value.as_str()),
        Some("UserError")
    );
}

#[tokio::test]
async fn test_error_namespace_inheritance() {
    crate::tst::logging();

    // Create namespace with error metadata
    let ctx = RefContext::new("test_package".to_string(), vec!["test".to_string()]);
    let ns_src = "#![err(ApiError)] namespace test;";
    let ns_def: Item<Namespace> = crate::tst::basic_smoke(ns_src).unwrap();
    let mut ns = NamespaceCtx {
        tag: None,
        ctx: ctx.clone(),
        sources: Default::default(),
        comments: vec![],
        error: {
            // Extract error from namespace meta
            ns_def
                .meta()
                .iter()
                .find_map(|meta_spanned| {
                    meta_spanned
                        .value
                        .meta
                        .iter()
                        .find_map(|item| {
                            if let crate::ast::meta::ItemMetaItem::Error(e) = item {
                                Some(
                                    e.clone()
                                        .with_source(PathBuf::from("test.ks")),
                                )
                            } else {
                                None
                            }
                        })
                })
        },
        version: None,
        namespace: ns_def.with_source("test.ks".into()),
        imports: Vec::new(),
        children: Default::default(),
        registry: TypeRegistry::new(),
        resolved_errors: Default::default(),
        resolved_versions: Default::default(),
        resolved_aliases: Default::default(),
    };

    // Add error type
    add_error_def(
        &mut ns,
        "ApiError",
        "error ApiError { UnknownUser { id: i64 } };",
    )
    .await;

    add_struct_def(&mut ns, "User", "struct User { id: i64 };").await;
    add_operation_def(&mut ns, "get_user", "operation get_user() -> User!;").await;

    let ns = crate::tst::register_namespace_types(ns)
        .await
        .expect("Failed to register types");

    let resolver = TypeResolver::new(ns);
    let resolution = resolver
        .resolve()
        .await
        .expect("Resolution should succeed");

    assert_eq!(
        resolution
            .errors
            .get("get_user")
            .map(|e| e.value.as_str()),
        Some("ApiError")
    );
}

#[tokio::test]
async fn test_error_missing_for_fallible_operation() {
    let resolver = resolver_with(vec![
        struct_def("User", "struct User { id: i64 };"),
        operation_def("create_user", "operation create_user() -> User!;"),
    ])
    .await;

    let result = resolver.resolve().await;

    // Should fail with MissingErrorType error
    assert!(result.is_err(), "Expected MissingErrorType error");
    if let Err(err) = result {
        assert!(
            format!("{}", err).contains("has no error type"),
            "Expected MissingErrorType error, got: {}",
            err
        );
    }
}

#[tokio::test]
async fn test_error_non_fallible_operation_no_error_needed() {
    crate::tst::logging();

    let resolver = resolver_with(vec![
        struct_def("User", "struct User { id: i64 };"),
        operation_def("get_user", "operation get_user() -> User;"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Should succeed for non-fallible operation");

    // Non-fallible operation should not have error in resolution
    assert!(resolution.errors.get("get_user").is_none());
}

#[tokio::test]
async fn test_error_parenthesized_result_type() {
    let resolver = resolver_with(vec![
        error_def("ApiError", "error ApiError { UserExists { id: i64 } };"),
        struct_def("User", "struct User { id: i64 };"),
        operation_def(
            "create_user",
            "#[err(ApiError)] operation create_user() -> (User)!;",
        ),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Should handle parenthesized result type");

    assert_eq!(
        resolution
            .errors
            .get("create_user")
            .map(|e| e.value.as_str()),
        Some("ApiError")
    );
}

// #[tokio::test]
// async fn test_valid_struct_reference() {
//     crate::tst::logging();

//     let ns = create_test_namespace("test");

//     // Add referenced struct
//     add_struct_def(&ns, "User", "struct User { id: i64 };").await;

//     // Add struct that references User
//     add_struct_def(&ns, "Post", "struct Post { author: User };").await;

//     let resolver = TypeResolver::new(ns.clone());
//     let resolution = resolver
//         .resolve()
//         .await
//         .expect("Valid references should succeed");

//     assert!(resolution.identified_structs.contains_key("User"));
//     assert!(resolution.identified_structs.contains_key("Post"));
// }

#[tokio::test]
async fn test_undefined_struct_reference() {
    let resolver = resolver_with(vec![struct_def("Post", "struct Post { author: User };")]).await;
    let result = resolver.resolve().await;

    assert!(result.is_err(), "Expected UndefinedType error");
    if let Err(err) = result {
        let err_msg = format!("{}", err);
        assert!(
            err_msg.contains("undefined type") || err_msg.contains("User"),
            "Expected UndefinedType error mentioning 'User', got: {}",
            err_msg
        );
    }
}

// #[tokio::test]
// async fn test_valid_operation_reference() {
//     crate::tst::logging();

//     let ns = create_test_namespace("test");

//     // Add referenced struct
//     add_struct_def(&ns, "User", "struct User { id: i64 };").await;

//     // Add operation that references User
//     add_operation_def(&ns, "GetUser", "operation GetUser(id: i64) -> User;").await;

//     let resolver = TypeResolver::new(ns.clone());
//     let resolution = resolver
//         .resolve()
//         .await
//         .expect("Valid operation references should succeed");

//     assert!(resolution.identified_structs.contains_key("User"));
//     assert!(resolution.operations.contains_key("GetUser"));
// }

#[tokio::test]
async fn test_undefined_operation_return_type() {
    crate::tst::logging();

    let resolver = resolver_with(vec![operation_def(
        "GetUser",
        "operation GetUser(id: i64) -> User;",
    )])
    .await;

    let result = resolver.resolve().await;

    // Should fail with UndefinedType error
    assert!(
        result.is_err(),
        "Expected UndefinedType error for return type"
    );
    if let Err(err) = result {
        let err_msg = format!("{}", err);
        assert!(
            err_msg.contains("undefined type") || err_msg.contains("User"),
            "Expected UndefinedType error mentioning 'User', got: {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_valid_oneof_reference() {
    let resolver = resolver_with(vec![
        struct_def("User", "struct User { id: i64 };"),
        struct_def("Admin", "struct Admin { id: i64 };"),
        oneof_def(
            "Actor",
            "oneof Actor { User (User), Admin { superuser: Admin } };",
        ),
    ])
    .await;

    let _ = resolver
        .resolve()
        .await
        .expect("Valid oneof references should succeed");
}

#[tokio::test]
async fn test_undefined_oneof_variant() {
    let resolver = resolver_with(vec![oneof_def(
        "Actor",
        "oneof Actor { User (User), Admin { superuser: Admin } };",
    )])
    .await;

    let result = resolver.resolve().await;

    // Should fail with UndefinedType error
    assert!(
        result.is_err(),
        "Expected UndefinedType error for oneof variant"
    );
    if let Err(err) = result {
        let err_msg = format!("{}", err);
        assert!(
            err_msg.contains("undefined type")
                || err_msg.contains("User")
                || err_msg.contains("Admin"),
            "Expected UndefinedType error mentioning variant, got: {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_nested_type_references() {
    let resolver = resolver_with(vec![
        struct_def("Address", "struct Address { street: str };"),
        struct_def("User", "struct User { address: Address };"),
        struct_def("Post", "struct Post { author: User };"),
    ])
    .await;

    let _ = resolver
        .resolve()
        .await
        .expect("Nested valid references should succeed");
}

#[tokio::test]
async fn test_array_type_reference() {
    let resolver = resolver_with(vec![
        struct_def("User", "struct User { id: i64 };"),
        struct_def("Users", "struct Users { items: User[] };"),
    ])
    .await;

    let _ = resolver
        .resolve()
        .await
        .expect("Array type references should succeed");
}

#[tokio::test]
async fn test_undefined_array_element_type() {
    let resolver =
        resolver_with(vec![struct_def("Users", "struct Users { items: User[] };")]).await;
    let result = resolver.resolve().await;

    // Should fail with UndefinedType error
    assert!(
        result.is_err(),
        "Expected UndefinedType error for array element"
    );
    if let Err(err) = result {
        let err_msg = format!("{}", err);
        assert!(
            err_msg.contains("undefined type") || err_msg.contains("User"),
            "Expected UndefinedType error mentioning 'User', got: {}",
            err_msg
        );
    }
}

// =============================================================================
// Union Or Tests (RFC-0016)
// =============================================================================

#[tokio::test]
async fn test_union_or_simple_merge() {
    // RFC-0016: Simple merge with non-conflicting fields
    let resolver = resolver_with(vec![
        struct_def("A", "struct A { foo: i32 };"),
        struct_def("B", "struct B { bar: str };"),
        type_alias("C", "type C = A &| B;"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Union Or resolution should succeed");

    // Verify the alias was resolved
    assert!(
        resolution.resolved_aliases.contains_key("C"),
        "C should be in resolved_aliases"
    );

    let resolved = &resolution.resolved_aliases["C"];

    // Verify it's a struct type
    if let Type::Struct { ty } = &resolved.value {
        let fields: Vec<_> = ty
            .value
            .fields
            .value
            .values
            .iter()
            .map(|f| f.value.name.borrow_string().clone())
            .collect();

        assert!(fields.contains(&"foo".to_string()), "Should have foo field");
        assert!(fields.contains(&"bar".to_string()), "Should have bar field");
        assert_eq!(fields.len(), 2, "Should have exactly 2 fields");
    } else {
        panic!("Expected struct type, got {:?}", resolved.value.type_name());
    }
}

#[tokio::test]
async fn test_union_or_conflict_creates_oneof() {
    // RFC-0016: Conflicting fields become oneof
    let resolver = resolver_with(vec![
        struct_def("A", "struct A { foo: i32 };"),
        struct_def("B", "struct B { foo: str };"),
        type_alias("C", "type C = A &| B;"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Union Or resolution should succeed");

    let resolved = &resolution.resolved_aliases["C"];

    if let Type::Struct { ty } = &resolved.value {
        assert_eq!(
            ty.value.fields.value.values.len(),
            1,
            "Should have 1 field (foo as oneof)"
        );

        let foo_field = &ty.value.fields.value.values[0];
        assert_eq!(foo_field.value.name.borrow_string(), "foo");

        // Verify foo is now a oneof type
        if let Type::OneOf { ty: oneof } = &foo_field.value.typ {
            assert_eq!(
                oneof.value.variants.value.values.len(),
                2,
                "Should have 2 variants: i32 and str"
            );
        } else {
            panic!(
                "Expected oneof type for conflicting field, got {:?}",
                foo_field.value.typ.type_name()
            );
        }
    } else {
        panic!("Expected struct type");
    }
}

#[tokio::test]
async fn test_union_or_chained_left_associative() {
    // RFC-0016: A &| B &| C is left-associative, parsed as (A &| B) &| C
    let resolver = resolver_with(vec![
        struct_def("A", "struct A { a: i32 };"),
        struct_def("B", "struct B { b: str };"),
        struct_def("C", "struct C { c: bool };"),
        type_alias("Combined", "type Combined = A &| B &| C;"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Chained Union Or should succeed");

    let resolved = &resolution.resolved_aliases["Combined"];

    if let Type::Struct { ty } = &resolved.value {
        let fields: Vec<_> = ty
            .value
            .fields
            .value
            .values
            .iter()
            .map(|f| f.value.name.borrow_string().clone())
            .collect();

        assert!(fields.contains(&"a".to_string()));
        assert!(fields.contains(&"b".to_string()));
        assert!(fields.contains(&"c".to_string()));
        assert_eq!(fields.len(), 3, "All three fields should be merged");
    } else {
        panic!("Expected struct type");
    }
}

#[tokio::test]
async fn test_union_or_with_duplicate_type_in_conflict() {
    // RFC-0016: Duplicate types in conflicts are deduplicated
    let resolver = resolver_with(vec![
        struct_def("A", "struct A { x: i32 };"),
        struct_def("B", "struct B { x: i32 };"), // Same type as A.x
        struct_def("C", "struct C { x: str };"), // Different type
        type_alias("D", "type D = A &| B &| C;"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Union Or with duplicates should succeed");

    let resolved = &resolution.resolved_aliases["D"];

    if let Type::Struct { ty } = &resolved.value {
        let x_field = &ty.value.fields.value.values[0];

        // x should be oneof with 2 variants (i32 and str), not 3
        if let Type::OneOf { ty: oneof } = &x_field.value.typ {
            assert_eq!(
                oneof.value.variants.value.values.len(),
                2,
                "Should dedupe to 2 variants (i32, str), not 3"
            );
        } else {
            panic!("Expected oneof type for x field");
        }
    } else {
        panic!("Expected struct type");
    }
}

#[tokio::test]
async fn test_union_or_non_struct_operand_error() {
    // RFC-0016: Non-struct operand should fail
    let resolver = resolver_with(vec![
        struct_def("A", "struct A { foo: i32 };"),
        enum_def("E", "enum E { X = 1 };"),
        type_alias("Invalid", "type Invalid = A &| E;"),
    ])
    .await;

    let result = resolver.resolve().await;

    assert!(result.is_err(), "Union Or with enum operand should fail");

    if let Err(crate::Error::Compiler(crate::CompilerError::Union(
        crate::UnionError::UnionOperandNotStruct { found_type, .. },
    ))) = result
    {
        assert_eq!(found_type, "enum");
    } else {
        panic!("Expected UnionOperandNotStruct error");
    }
}

#[tokio::test]
async fn test_union_or_with_anonymous_struct() {
    // RFC-0016: Anonymous struct operands
    let resolver = resolver_with(vec![
        struct_def("A", "struct A { a: i32 };"),
        type_alias("Combined", "type Combined = A &| { b: str };"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Union Or with anonymous struct should succeed");

    let resolved = &resolution.resolved_aliases["Combined"];

    if let Type::Struct { ty } = &resolved.value {
        let fields: Vec<_> = ty
            .value
            .fields
            .value
            .values
            .iter()
            .map(|f| f.value.name.borrow_string().clone())
            .collect();

        assert!(fields.contains(&"a".to_string()));
        assert!(fields.contains(&"b".to_string()));
    } else {
        panic!("Expected struct type");
    }
}

#[tokio::test]
async fn test_union_or_preserves_field_optionality() {
    // Verify optional fields are preserved
    let resolver = resolver_with(vec![
        struct_def("A", "struct A { required: i32, optional?: str };"),
        struct_def("B", "struct B { other: bool };"),
        type_alias("Combined", "type Combined = A &| B;"),
    ])
    .await;

    let resolution = resolver
        .resolve()
        .await
        .expect("Resolution should succeed");

    let resolved = &resolution.resolved_aliases["Combined"];

    if let Type::Struct { ty } = &resolved.value {
        assert_eq!(ty.value.fields.value.values.len(), 3);
    } else {
        panic!("Expected struct type");
    }
}
