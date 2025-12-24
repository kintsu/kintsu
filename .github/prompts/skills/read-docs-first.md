---
name: read-docs-first
title: "Read Documentation First"
description: "Simple but critical rule: before editing any code, ensure you read all relevant documentation in docs/src/**/*.md and review schema examples in parser/samples/**/*.ks."
tags: ["kintsu", "workflow", "documentation", "best-practices"]
updated: 2025-12-22
---
# Read Documentation First

Before editing any code, ensure you read all relevant documentation.

## Required Reading

1. **Target State Documentation**: `docs/src/**/*.md`
   - Understand what we are trying to achieve
   - Review design specifications and requirements

2. **Schema Examples**: `parser/samples/**/*.ks`
   - Review actual schema syntax examples
   - Understand expected input formats

## Workflow

1. Read docs to understand target state
2. Review examples to understand syntax
3. Only then proceed with code changes

## Purpose

This ensures changes align with:
- Documented design decisions
- Expected behavior
- Existing conventions