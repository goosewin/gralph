# Backends

Gralph supports multiple AI coding assistants.

## Claude Code (Default)

```bash
npm install -g @anthropic-ai/claude-code
gralph start .
gralph start . --backend claude
```

**Models:** `claude-opus-4-5`

## OpenCode

```bash
npm install -g opencode-ai
gralph start . --backend opencode
gralph start . --backend opencode --model google/gemini-1.5-pro
```

**Models (provider/model format):**
- `opencode/example-code-model`
- `anthropic/claude-opus-4-5`
- `google/gemini-1.5-pro`

## Gemini CLI

```bash
npm install -g @google/gemini-cli
gralph start . --backend gemini
```

**Models:** `gemini-1.5-pro`

## Codex CLI

```bash
npm install -g @openai/codex
gralph start . --backend codex
```

**Models:** `example-codex-model`

## Setting Default Backend

Config file:
```yaml
defaults:
  backend: opencode
  model: opencode/example-code-model
```

Environment:
```bash
export GRALPH_DEFAULTS_BACKEND=opencode
```

## Check Installed Backends

```bash
gralph backends
```
