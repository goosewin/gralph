namespace Gralph.Core;

public static class PromptTemplates
{
    public const string Default =
        "Read {task_file} carefully. Find any task marked '- [ ]' (unchecked).\n" +
        "\n" +
        "If unchecked tasks exist:\n" +
        "- Complete ONE task fully\n" +
        "- Mark it '- [x]' in {task_file}\n" +
        "- Commit changes\n" +
        "- Exit normally (do NOT output completion promise)\n" +
        "\n" +
        "If ZERO '- [ ]' remain (all complete):\n" +
        "- Verify by searching the file\n" +
        "- Output ONLY: <promise>{completion_marker}</promise>\n" +
        "\n" +
        "CRITICAL: Never mention the promise unless outputting it as the completion signal.\n" +
        "\n" +
        "{context_files_section}" +
        "Task Block:\n" +
        "{task_block}\n" +
        "\n" +
        "Iteration: {iteration}/{max_iterations}";
}
