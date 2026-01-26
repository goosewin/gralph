using System;
using System.Collections.Generic;
using System.Linq;

namespace Gralph.Prd;

public sealed class PrdStackSummary
{
    public PrdStackSummary(
        IReadOnlyList<string> stackIds,
        IReadOnlyList<string> languages,
        IReadOnlyList<string> frameworks,
        IReadOnlyList<string> tools,
        IReadOnlyList<string> runtimes,
        IReadOnlyList<string> packageManagers,
        IReadOnlyList<string> evidence,
        IReadOnlyList<string> selectedStackIds)
    {
        StackIds = stackIds;
        Languages = languages;
        Frameworks = frameworks;
        Tools = tools;
        Runtimes = runtimes;
        PackageManagers = packageManagers;
        Evidence = evidence;
        SelectedStackIds = selectedStackIds;
    }

    public IReadOnlyList<string> StackIds { get; }
    public IReadOnlyList<string> Languages { get; }
    public IReadOnlyList<string> Frameworks { get; }
    public IReadOnlyList<string> Tools { get; }
    public IReadOnlyList<string> Runtimes { get; }
    public IReadOnlyList<string> PackageManagers { get; }
    public IReadOnlyList<string> Evidence { get; }
    public IReadOnlyList<string> SelectedStackIds { get; }

    public string FormatSummary(int headingLevel = 2)
    {
        var prefix = headingLevel == 1 ? "#" : "##";
        var lines = new List<string>
        {
            $"{prefix} Stack Summary",
            string.Empty,
            $"- Stacks: {FormatList(StackIds, "Unknown")}",
            $"- Languages: {FormatList(Languages, "Unknown")}",
            $"- Runtimes: {FormatList(Runtimes, "Unknown")}",
            $"- Frameworks: {FormatList(Frameworks, "None detected")}",
            $"- Tools: {FormatList(Tools, "None detected")}",
            $"- Package managers: {FormatList(PackageManagers, "None detected")}",
        };

        if (SelectedStackIds.Count > 0 && SelectedStackIds.Count < StackIds.Count)
        {
            lines.Add($"- Stack focus: {string.Join(", ", SelectedStackIds)}");
        }

        lines.Add(string.Empty);
        lines.Add("Evidence:");
        if (Evidence.Count > 0)
        {
            lines.AddRange(Evidence.Select(item => $"- {item}"));
        }
        else
        {
            lines.Add("- None found");
        }

        return string.Join("\n", lines);
    }

    private static string FormatList(IReadOnlyList<string> items, string fallback)
    {
        return items.Count > 0 ? string.Join(", ", items) : fallback;
    }
}
