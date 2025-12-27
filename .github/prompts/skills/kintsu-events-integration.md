---
name: kintsu-events-integration
title: "Kintsu Events System Integration"
description: "Guide for integrating kintsu-events diagnostic emission across crate boundaries. Covers initialization, warning emission, and test harness patterns."
tags: ["kintsu", "events", "error-handling", "integration"]
updated: 2025-12-27
---

# Kintsu Events Integration

Guide for integrating the diagnostic emission system across crate boundaries.

## Initialization Pattern

```rust
// CLI entry point
use kintsu_events::{DiagnosticBundle, StderrReporter};

rt.block_on(async {
    kintsu_events::init(vec![Box::new(StderrReporter)]);

    let result = cli.run().await;
    let bundle = kintsu_events::shutdown().await;

    match result {
        Ok(()) if bundle.has_errors() => ExitCode::FAILURE,
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            kintsu_events::emit_error(e);
            ExitCode::FAILURE
        },
    }
})
```

## Warning Emission Pattern

```rust
use kintsu_events;

// Emit warning without stopping compilation
let warning = SomeError::warning_variant(args)
    .at(span)
    .build()
    .with_source_arc_if(path, content);

kintsu_events::emit_warning(warning);
// Compilation continues - warning collected in bundle
```

## Dependency Setup

```toml
# In crate that needs to emit
[dependencies]
kintsu-events = { path = "../events" }
```

## Test Harness

```rust
// For warnings (expect success + code present)
CliErrorTest::new("test_name")
    .expect_warning("KUN3001")  // Not expect_error
    .requires_span(true)
    .run_and_assert();
```

## Key Files

- Events API: events/src/lib.rs
- Diagnostic type: events/src/diagnostic.rs
- Reporter trait: events/src/reporter.rs
- Union warnings: parser/src/ctx/resolve/union_or.rs
- CLI init: cli/src/bin/kintsu.rs
