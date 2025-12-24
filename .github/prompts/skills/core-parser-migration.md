---
name: core-parser-migration
title: "Migrate Core Types to Parser Declarations"
description: "Documents the end-to-end migration of kintsu-core from internal type declarations to parser DeclarationVersion types"
tags: ["kintsu", "migration", "core", "parser", "rust"]
updated: 2025-12-24
---
# Core to Parser Declarations Migration

## Overview

This skill documents how to migrate kintsu-core from internal type declarations to use `DeclarationVersion` from kintsu-parser.

## Approach

Use backward-compatible parallel API strategy:
- Keep existing `Defined` trait and `Definitions` enum
- Add new `DeclDefined` trait returning `TypeDefinition`
- Blanket impl connects old to new via conversion

## Files Created

1. **core/src/convert.rs** - `From` implementations:
   - `Type` → `DeclType`
   - `Struct` → `DeclStruct`
   - `Enum` → `DeclEnumDef`
   - `OneOf` → `DeclOneOf`
   - `ErrorTy` → `DeclError`
   - `Operation` → `DeclOperation`
   - `Definitions` → `TypeDefinition`
   - `Namespace` → `DeclNamespace`

2. **core/src/generate/decl_ext.rs** - Extension traits:
   - `DeclTypeExt::to_rust_tokens()`
   - `BuiltinExt` for primitive types
   - `DeclFieldExt::to_rust_field()`
   - Helper functions for comments/docs

3. **core/src/generate/decl_gen.rs** - Generation traits:
   - `GenerateDecl` trait for parser types
   - `DeclNsContext` context structure

4. **core/src/generate/rust_decl.rs** - Rust implementation:
   - `impl GenerateDecl for RustGenerator`

## Modified Files

1. **core/src/lib.rs**:
   - Added `declare` module re-exports
   - Added `DeclDefined` trait with blanket impl

2. **core/src/ty.rs**:
   - Added `Version::get()` for public access

3. **core/src/context.rs**:
   - Added `to_declaration_bundle()` method
   - Added `DeclContext` struct

4. **sdk/src/lib.rs**:
   - Export `DeclDefined` trait
   - Export `declare` module

## Key Pattern

```rust
pub trait DeclDefined: Sized + Send + Sync {
    fn type_definition() -> declare::TypeDefinition;
}

impl<T: Defined> DeclDefined for T {
    fn type_definition() -> declare::TypeDefinition {
        T::definition().into()
    }
}
```

## Testing

```bash
cargo test -p kintsu-core --features "generate,chrono"  # 96 tests
cargo test -p kintsu-sdk  # 7 tests
```

## Notes

- Parser types use `DeclComment` not `Meta` for documentation
- `DeclNamespace.version` is `Option<u32>` vs core's `Version` wrapper
- Use `.values()` iteration pattern per clippy guidance
