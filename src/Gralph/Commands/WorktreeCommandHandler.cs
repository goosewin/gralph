using System.Diagnostics;
using System.Text.RegularExpressions;

namespace Gralph.Commands;

public sealed class WorktreeCommandHandler
{
    private static readonly Regex TaskIdPattern = new("^[A-Za-z]+-[0-9]+$", RegexOptions.Compiled);

    public int ExecuteCreate(string? taskId)
    {
        if (!TryValidateTaskId(taskId, "create"))
        {
            return 1;
        }

        if (!TryResolveRepoRoot(out var repoRoot))
        {
            return 1;
        }

        if (!EnsureClean(repoRoot))
        {
            return 1;
        }

        var branchName = $"task-{taskId}";
        var worktreesDir = Path.Combine(repoRoot, ".worktrees");
        Directory.CreateDirectory(worktreesDir);

        var worktreePath = Path.Combine(worktreesDir, branchName);
        if (BranchExists(repoRoot, branchName))
        {
            Console.Error.WriteLine($"Error: Branch already exists: {branchName}");
            return 1;
        }

        if (Directory.Exists(worktreePath) || File.Exists(worktreePath))
        {
            Console.Error.WriteLine($"Error: Worktree path already exists: {worktreePath}");
            return 1;
        }

        var exitCode = RunGit(repoRoot, out _, out var error, "worktree", "add", "-b", branchName, worktreePath);
        if (exitCode != 0)
        {
            Console.Error.WriteLine($"Error: Failed to create worktree at {worktreePath}");
            if (!string.IsNullOrWhiteSpace(error))
            {
                Console.Error.WriteLine(error.Trim());
            }
            return 1;
        }

        Console.WriteLine($"Created worktree {worktreePath} on branch {branchName}");
        return 0;
    }

    public int ExecuteFinish(string? taskId)
    {
        if (!TryValidateTaskId(taskId, "finish"))
        {
            return 1;
        }

        if (!TryResolveRepoRoot(out var repoRoot))
        {
            return 1;
        }

        if (!EnsureClean(repoRoot))
        {
            return 1;
        }

        var branchName = $"task-{taskId}";
        var worktreesDir = Path.Combine(repoRoot, ".worktrees");
        var worktreePath = Path.Combine(worktreesDir, branchName);

        if (!BranchExists(repoRoot, branchName))
        {
            Console.Error.WriteLine($"Error: Branch does not exist: {branchName}");
            return 1;
        }

        if (!Directory.Exists(worktreePath))
        {
            Console.Error.WriteLine($"Error: Worktree path is missing: {worktreePath} (run 'gralph worktree create {taskId}' first)");
            return 1;
        }

        var currentBranch = GetCurrentBranch(repoRoot);
        if (string.Equals(currentBranch, branchName, StringComparison.OrdinalIgnoreCase))
        {
            Console.Error.WriteLine($"Error: Cannot finish while on branch {branchName}");
            return 1;
        }

        var mergeExitCode = RunGit(repoRoot, out _, out var mergeError, "merge", "--no-ff", branchName);
        if (mergeExitCode != 0)
        {
            Console.Error.WriteLine($"Error: Failed to merge branch: {branchName}");
            if (!string.IsNullOrWhiteSpace(mergeError))
            {
                Console.Error.WriteLine(mergeError.Trim());
            }
            return 1;
        }

        var removeExitCode = RunGit(repoRoot, out _, out var removeError, "worktree", "remove", worktreePath);
        if (removeExitCode != 0)
        {
            Console.Error.WriteLine($"Error: Failed to remove worktree at {worktreePath}");
            if (!string.IsNullOrWhiteSpace(removeError))
            {
                Console.Error.WriteLine(removeError.Trim());
            }
            return 1;
        }

        Console.WriteLine($"Finished worktree {worktreePath} and merged {branchName}");
        return 0;
    }

    private static bool TryValidateTaskId(string? taskId, string action)
    {
        if (string.IsNullOrWhiteSpace(taskId))
        {
            Console.Error.WriteLine($"Error: Usage: gralph worktree {action} <ID>");
            return false;
        }

        if (!TaskIdPattern.IsMatch(taskId))
        {
            Console.Error.WriteLine($"Error: Invalid task ID format: {taskId} (expected like A-1)");
            return false;
        }

        return true;
    }

    private static bool TryResolveRepoRoot(out string repoRoot)
    {
        repoRoot = string.Empty;
        var exitCode = RunGit(Environment.CurrentDirectory, out var output, out _, "rev-parse", "--show-toplevel");
        if (exitCode != 0)
        {
            Console.Error.WriteLine("Error: Not a git repository (or any of the parent directories)");
            return false;
        }

        repoRoot = output.Trim();
        return !string.IsNullOrWhiteSpace(repoRoot);
    }

    private static bool EnsureClean(string repoRoot)
    {
        var exitCode = RunGit(repoRoot, out var output, out _, "status", "--porcelain");
        if (exitCode != 0)
        {
            Console.Error.WriteLine($"Error: Unable to check git status in {repoRoot}");
            return false;
        }

        if (!string.IsNullOrWhiteSpace(output))
        {
            Console.Error.WriteLine("Error: Git working tree is dirty. Commit or stash changes before running worktree commands.");
            return false;
        }

        return true;
    }

    private static bool BranchExists(string repoRoot, string branchName)
    {
        var exitCode = RunGit(repoRoot, out _, out _, "show-ref", "--verify", "--quiet", $"refs/heads/{branchName}");
        return exitCode == 0;
    }

    private static string GetCurrentBranch(string repoRoot)
    {
        var exitCode = RunGit(repoRoot, out var output, out _, "rev-parse", "--abbrev-ref", "HEAD");
        return exitCode == 0 ? output.Trim() : string.Empty;
    }

    private static int RunGit(string? workingDirectory, out string stdout, out string stderr, params string[] args)
    {
        stdout = string.Empty;
        stderr = string.Empty;

        var startInfo = new ProcessStartInfo
        {
            FileName = "git",
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true
        };

        if (!string.IsNullOrWhiteSpace(workingDirectory))
        {
            startInfo.WorkingDirectory = workingDirectory;
        }

        foreach (var arg in args)
        {
            startInfo.ArgumentList.Add(arg);
        }

        using var process = Process.Start(startInfo);
        if (process is null)
        {
            stderr = "Failed to start git process.";
            return 1;
        }

        stdout = process.StandardOutput.ReadToEnd();
        stderr = process.StandardError.ReadToEnd();
        process.WaitForExit();
        return process.ExitCode;
    }
}
