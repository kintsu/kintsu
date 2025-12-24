# Agent Instructions

This project uses Factsets for persistent knowledge management.

## Project Overview

Kintsu is a new-generation type system for messaging transport declarations and code generation. It's implemented as a Rust workspace with 21 crates covering parsing, code generation, registry services, and tooling.

## Required Behavior

- Use Factsets tools on EVERY prompt without exception
- Never wait for user to ask - be proactive
- Validate context before responding: `search_facts`, `search_skills`
- Capture all learnings immediately: `submit_facts`
- Register all fetched URLs: `add_resources`
- Log all successful commands: `submit_execution_logs`
- Check preferences before generating output: `get_preference_prompt`

## On Every Prompt

Before responding to any user message:

1. `search_facts` with tags derived from the prompt topic
2. `search_skills` if the prompt involves a procedure or "how to"

## During Long Tasks

For multi-step work, use Factsets between steps:

- After each successful command: `submit_execution_logs`
- After discovering something: `submit_facts`
- After fetching external content: `add_resources`

Do not wait until the task is complete - context windows grow large and knowledge gets lost.

## Automatic Capture Triggers

| Event                            | Action                       |
| -------------------------------- | ---------------------------- |
| You learn something              | `submit_facts` immediately   |
| You fetch a URL                  | `add_resources` with the URL |
| A command succeeds               | `submit_execution_logs`      |
| You write a multi-step procedure | `create_skill`               |
| User corrects your output        | `infer_preference`           |

## Key Project Tags

When searching or submitting to Factsets, use these tags:

- `kintsu` - General project context
- `parser` - Lexing, parsing, AST, .ks schema files
- `core` - Type system, code generation
- `registry` - Web service, actix-web, sea-orm
- `rust` - Rust-specific patterns and conventions
- `syntax` - Kintsu DSL syntax (structs, enums, operations, types)
- `error-handling` - miette, thiserror patterns
- `workspace` - Cargo workspace configuration

## Build Verification

Always verify code changes compile:

```bash
cargo build -p {packageName}
```

## Code Style

Follow the conventions in `.github/prompts/rust.md`:

- Prefer borrowing (`&T`) over cloning
- Use `Result<T, E>` for recoverable errors in library code
- Use iterators instead of index-based loops
- Brief comments using simple dashes (-), not em dashes
- Use bon::Builder for complex object construction

## No Reminders Needed

Agents must use Factsets automatically. Users should never need to remind
agents to use the knowledge base. If you are not using Factsets on every
prompt, you are doing it wrong. Call `get_agent_guide` for detailed guidance.

## Kintsu-Specific Knowledge

### Schema Syntax (.ks files)

```
// Namespace with version
#![version(1)]
namespace my_namespace;

// Struct definition
struct MyStruct {
    field1: i32,
    field2?: str,  // optional field
};

// Enum definition
enum MyEnum {
    A = 1,
    B = 42
};

// Union types
type MyUnion = oneof str | i32;

// Operations (fallible with !)
operation my_op(input: str) -> MyStruct!;
operation add(a: i32, b: i32) -> i32;  // infallible

// Error types
error MyError {
    Unknown { desc: str },
    Code(i32)
};
```

### Key Dependencies

- `miette` - Diagnostic error reporting with fancy output
- `logos` - Lexer generator for tokenization
- `thiserror` - Error type derivation
- `actix-web` - Async web framework for registry
- `sea-orm` - Async ORM for database access
- `tokio` - Async runtime
- `rayon` - Data parallelism
- `bon` - Builder pattern macro
