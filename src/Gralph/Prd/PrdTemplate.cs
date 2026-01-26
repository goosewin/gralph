using System;
using System.IO;

namespace Gralph.Prd;

public static class PrdTemplate
{
    public static string GetTemplateText(string targetDir)
    {
        if (!string.IsNullOrWhiteSpace(targetDir))
        {
            var candidate = Path.Combine(targetDir, "PRD.template.md");
            if (File.Exists(candidate))
            {
                return File.ReadAllText(candidate);
            }
        }

        var cwdCandidate = Path.Combine(Directory.GetCurrentDirectory(), "PRD.template.md");
        if (File.Exists(cwdCandidate))
        {
            return File.ReadAllText(cwdCandidate);
        }

        return DefaultTemplate;
    }

    private const string DefaultTemplate = "## Overview\n\n" +
                                          "Briefly describe the project, goals, and intended users.\n\n" +
                                          "## Problem Statement\n\n" +
                                          "- What problem does this solve?\n" +
                                          "- What pain points exist today?\n\n" +
                                          "## Solution\n\n" +
                                          "High-level solution summary.\n\n" +
                                          "---\n\n" +
                                          "## Functional Requirements\n\n" +
                                          "### FR-1: Core Feature\n\n" +
                                          "Describe the primary user-facing behavior.\n\n" +
                                          "### FR-2: Secondary Feature\n\n" +
                                          "Describe supporting behavior.\n\n" +
                                          "---\n\n" +
                                          "## Non-Functional Requirements\n\n" +
                                          "### NFR-1: Performance\n\n" +
                                          "- Example: Response times under 200ms for key operations.\n\n" +
                                          "### NFR-2: Reliability\n\n" +
                                          "- Example: Crash recovery or retries where appropriate.\n\n" +
                                          "---\n\n" +
                                          "## Implementation Tasks\n\n" +
                                          "Each task must use a `### Task <ID>` block header and include the required fields.\n" +
                                          "Each task block must contain exactly one unchecked task line.\n\n" +
                                          "### Task EX-1\n\n" +
                                          "- **ID** EX-1\n" +
                                          "- **Context Bundle** `path/to/file`, `path/to/other`\n" +
                                          "- **DoD** Define the done criteria for this task.\n" +
                                          "- **Checklist**\n" +
                                          "  * First verification item.\n" +
                                          "  * Second verification item.\n" +
                                          "- **Dependencies** None\n" +
                                          "- [ ] EX-1 Implement core feature\n";
}
