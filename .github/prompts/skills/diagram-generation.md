---
name: diagram-generation
title: "Diagram Generation with Python Diagrams"
description: "Generate implementation diagrams programmatically using the Python diagrams library. Covers preferred providers (programming, generic, c4), workflow steps, and strategies for breaking down large diagrams into helper functions."
tags: ["kintsu", "documentation", "diagrams", "python", "visualization"]
updated: 2025-12-22
---
# Diagram Generation with Python Diagrams

Generate implementation diagrams programmatically using the Python `diagrams` library.

## Library Reference

- Documentation: https://diagrams.mingrammer.com/
- Source code: https://github.com/mingrammer/diagrams (under `docs/`)

## Diagram Types

For `docs/diagrams/*`, prefer these providers:
- `diagrams.programming` - Programming language and tool icons
- `diagrams.generic` - Generic computing icons
- `diagrams.c4` - C4 model architecture diagrams

**Avoid** using cloud-specific diagrams (like `s3`) unless explicitly required.

## Workflow

1. **Analyze** the system/code before generating
2. **Ensure accuracy** - represent what actually exists
3. **Generate** the `.py` file
4. **Run** to create PNG: `python -m docs.diagrams.{diagram_name}`

## Large Diagram Strategy

For complex diagrams:

1. Break into smaller helper functions
2. Call helpers within the main diagram scope
3. Create separate smaller diagrams that only call individual helpers
4. This ensures sub-graphs exist for all larger comprehensive graphs

## Example Structure

```python
from diagrams import Diagram, Cluster
from diagrams.programming.language import Rust
from diagrams.generic.compute import Rack

def parser_cluster():
    """Helper for parser subsystem."""
    with Cluster("Parser"):
        lexer = Rack("Lexer")
        ast = Rack("AST")
        lexer >> ast
    return lexer, ast

def main_diagram():
    """Main comprehensive diagram."""
    with Diagram("Kintsu Architecture", show=False):
        parser_start, parser_end = parser_cluster()
        # ... more components

# Also create focused sub-diagram
def parser_diagram():
    """Focused parser-only diagram."""
    with Diagram("Parser Subsystem", show=False):
        parser_cluster()
```

## Best Practices

- Always analyze before generating
- Represent the _current_ state accurately
- Use informed analysis from code review
- Follow rules in quantum-thinking workflow for thorough analysis