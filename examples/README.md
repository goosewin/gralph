# Example PRDs

This directory holds a small, self-contained set of example PRDs that
demonstrate the current schema and the expected task block format.

## Example files
- PRD-Stage-P-Example.md: Minimal PRD for parser and prompt wiring tasks.
- PRD-Stage-A-Example.md: Example PRD for shared doc creation tasks.
- README.md: Describes the example set and how to run it.

## Run the examples
Run the examples in order from the repo root:

```sh
gralph start ./gralph --task-file gralph/examples/PRD-Stage-P-Example.md --no-tmux --backend claude --model claude-opus-4-5 && \
  gralph start ./gralph --task-file gralph/examples/PRD-Stage-A-Example.md --no-tmux --backend claude --model claude-opus-4-5
```
