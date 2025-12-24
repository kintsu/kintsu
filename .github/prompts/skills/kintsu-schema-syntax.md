# Kintsu Schema Syntax Guide

This skill documents the Kintsu schema language (.ks files) syntax and conventions.

## File Structure

Every Kintsu schema package must have a `lib.ks` file at the root of the `schema/` directory containing:
- Namespace declaration
- Use statements for imports

```
schema/
  lib.ks          # Required: namespace and imports
  types.ks        # Optional: type definitions
  operations.ks   # Optional: operation definitions
```

## Namespace Declaration

```kintsu
// Version attribute (optional)
#![version(1)]

// Namespace declaration (required)
namespace my_namespace;

// Imports
use other_ns;
```

## Primitive Types

| Type | Description |
|------|-------------|
| `i8`, `i16`, `i32`, `i64` | Signed integers |
| `u8`, `u16`, `u32`, `u64` | Unsigned integers |
| `f32`, `f64` | Floating point |
| `str` | String |
| `bool` | Boolean |

## Struct Definitions

```kintsu
struct MyStruct {
    required_field: i32,
    optional_field?: str,
    array_field: [i32],
    sized_array: [i32; 4],
};
```

- Use `?` suffix for optional fields
- Arrays use `[T]` syntax
- Sized arrays use `[T; N]` syntax
- Semi-colon required after closing brace

## Enum Definitions

```kintsu
// Integer discriminants
enum IntEnum {
    A = 1,
    B = 42
};

// String discriminants
enum StrEnum {
    A = "a_val",
    B = "b_val"
};

// Default (auto-incrementing)
enum DefaultEnum {
    A,
    B,
    C
};
```

## Union Types (oneof)

```kintsu
type MyUnion = oneof str | i32 | bool;

// Can be used in struct fields
struct Container {
    value: oneof str | i32
};
```

## Error Definitions

```kintsu
error MyError {
    // Struct-style variant
    NotFound {
        resource: str
    },
    // Tuple-style variant
    Code(i32),
    // Unit variant
    Unknown
};
```

## Operations

```kintsu
// Namespace-level error type
#![err(MyError)]
namespace foo;

// Fallible operation (returns Result)
operation fetch_data(id: str) -> Data!;

// Infallible operation (cannot fail)
operation add(a: i32, b: i32) -> i32;

// Multiple parameters
operation process(input: str, options: Options) -> Output!;
```

- `!` suffix indicates fallible operation
- Fallible operations require error type to be defined
- Parameters are comma-separated

## Cross-Package References

```kintsu
use other_package;

struct MyStruct {
    // Reference type from another package
    external_data: other_package::namespace::ExternalType
};
```

## Comments

```kintsu
// Single-line comment

/* 
   Multi-line
   comment
*/

/// Documentation comment for items below
struct Documented {};
```

## Best Practices

1. Keep namespaces focused and cohesive
2. Use descriptive type names in PascalCase
3. Use snake_case for field names
4. Group related operations together
5. Define error types specific to each namespace
6. Use optional fields (`?`) sparingly - prefer defaults when possible
7. Document complex types with comments
