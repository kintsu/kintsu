# Factsets Quickstart Guide

Welcome to Factsets! This guide will help you get started with building your knowledge base for the Kintsu project.

## Core Concepts

### Facts
Facts are atomic pieces of information. They should be:
- **Self-contained**: Understandable without additional context
- **Verifiable**: Can be marked as verified once confirmed
- **Tagged**: Associated with relevant tags for organization
- **1-3 sentences**: Keep them concise and focused

### Tags
Tags organize your knowledge into meaningful groups. For Kintsu, use:
- `kintsu` - General project context
- `parser`, `core`, `registry` - Crate-specific tags
- `syntax`, `operations`, `types` - Schema language concepts
- `rust`, `error-handling`, `patterns` - Implementation details
- `documentation`, `deployment`, `testing` - Development workflow

### Skills
Skills are markdown documents capturing procedural knowledge:
- How to perform specific tasks
- Best practices and patterns
- Workflows and processes

### Resources
Resources track external content with retrieval methods:
- Files in your workspace (type: `file`)
- URLs and documentation (type: `url`)
- API endpoints (type: `api`)
- Shell commands (type: `command`)

## Kintsu-Specific Workflow

1. **Before any task**: Search for existing knowledge
   ```json
   { "tool": "search_facts", "tags": ["kintsu", "relevant-topic"] }
   ```

2. **When learning something**: Submit facts immediately
   ```json
   {
     "facts": [{
       "content": "Description of what you learned",
       "sourceType": "code",
       "tags": ["kintsu", "relevant-tags"],
       "verified": true
     }]
   }
   ```

3. **When fetching URLs**: Register as resources
   ```json
   {
     "resources": [{
       "uri": "https://docs.rs/...",
       "type": "url",
       "tags": ["rust", "documentation"],
       "description": "What this resource covers",
       "retrievalMethod": { "type": "url", "url": "..." }
     }]
   }
   ```

4. **When commands succeed**: Log them for institutional memory

## Agent Interaction

If you're an AI agent, call `get_agent_guide` first for comprehensive workflow instructions.

Use Factsets on EVERY prompt - not just at session start. The system is free and cannot be overwhelmed.

## Tips

- Start with facts for discrete information
- Graduate to skills for procedural knowledge
- Link skills to related facts, resources, and other skills
- Use tags consistently across your knowledge base
- Verify facts when you confirm they are accurate
- Update facts when information changes
