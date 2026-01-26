using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Linq;
using Gralph.Backends;
using Gralph.Configuration;

namespace Gralph.Prd;

public sealed class PrdCreateSettings
{
    public string? Directory { get; init; }
    public string? Output { get; init; }
    public string? Goal { get; init; }
    public string? Constraints { get; init; }
    public string? Context { get; init; }
    public string? Sources { get; init; }
    public string? Backend { get; init; }
    public string? Model { get; init; }
    public bool AllowMissingContext { get; init; }
    public bool Multiline { get; init; }
    public bool Force { get; init; }
    public bool? Interactive { get; init; }
}

public static class PrdGenerator
{
    public static async Task<int> GenerateAsync(PrdCreateSettings settings, BackendRegistry backendRegistry, CancellationToken cancellationToken)
    {
        if (settings is null)
        {
            throw new ArgumentNullException(nameof(settings));
        }

        if (backendRegistry is null)
        {
            throw new ArgumentNullException(nameof(backendRegistry));
        }

        var targetDir = ResolveTargetDir(settings.Directory);
        if (string.IsNullOrWhiteSpace(targetDir) || !Directory.Exists(targetDir))
        {
            Console.Error.WriteLine($"Error: Directory does not exist: {targetDir}");
            return 1;
        }

        Config.Load(targetDir);

        var interactive = ResolveInteractive(settings.Interactive);
        if (interactive)
        {
            Console.Error.WriteLine("Interactive mode: follow the numbered steps. Press Enter to skip optional prompts.");
        }

        var goal = settings.Goal;
        if (string.IsNullOrWhiteSpace(goal) && interactive)
        {
            Console.Error.WriteLine("Step 1/6: Project goal (required). Press Enter to skip if already provided.");
            goal = settings.Multiline ? PromptMultiline("Goal (required)") : PromptInput("Goal (required)", string.Empty);
        }

        if (string.IsNullOrWhiteSpace(goal))
        {
            Console.Error.WriteLine("Error: Goal is required. Use --goal or run interactively.");
            return 1;
        }

        var constraints = settings.Constraints;
        if (string.IsNullOrWhiteSpace(constraints) && interactive)
        {
            Console.Error.WriteLine("Step 2/6: Constraints or requirements (optional). Press Enter to skip.");
            constraints = settings.Multiline ? PromptMultiline("Constraints (optional)") : PromptInput("Constraints (optional)", string.Empty);
        }

        if (string.IsNullOrWhiteSpace(constraints))
        {
            constraints = "None.";
        }

        var sourcesInput = settings.Sources;
        if (string.IsNullOrWhiteSpace(sourcesInput) && interactive)
        {
            Console.Error.WriteLine("Step 3/6: External sources (comma-separated URLs). Press Enter to skip.");
            sourcesInput = PromptInput("Sources (optional)", string.Empty);
        }

        var outputPath = settings.Output;
        if (string.IsNullOrWhiteSpace(outputPath) && interactive)
        {
            Console.Error.WriteLine("Step 4/6: Output file (press Enter for PRD.generated.md).");
            outputPath = PromptInput("PRD output file", "PRD.generated.md");
        }

        if (string.IsNullOrWhiteSpace(outputPath))
        {
            outputPath = "PRD.generated.md";
        }

        if (!Path.IsPathRooted(outputPath))
        {
            outputPath = Path.Combine(targetDir, outputPath);
        }

        if (File.Exists(outputPath) && !settings.Force)
        {
            if (interactive)
            {
                var overwrite = PromptInput("File exists. Overwrite? (y/N)", "N");
                if (!string.Equals(overwrite, "y", StringComparison.OrdinalIgnoreCase))
                {
                    Console.Error.WriteLine($"Error: Output file exists: {outputPath} (use --force to overwrite)");
                    return 1;
                }
            }
            else
            {
                Console.Error.WriteLine($"Error: Output file exists: {outputPath} (use --force to overwrite)");
                return 1;
            }
        }

        var backendName = settings.Backend;
        if (string.IsNullOrWhiteSpace(backendName))
        {
            backendName = Config.Get("defaults.backend", BackendRegistry.DefaultBackendName);
        }

        if (!backendRegistry.TryGet(backendName, out var backend) || backend is null)
        {
            var available = string.Join(", ", backendRegistry.List().Select(item => item.Name));
            Console.Error.WriteLine($"Error: Unknown backend '{backendName}'. Available backends: {available}");
            return 1;
        }

        if (!backend.IsInstalled())
        {
            Console.Error.WriteLine($"Error: Backend '{backendName}' CLI is not installed");
            Console.Error.WriteLine($"Install with: {backend.GetInstallHint()}");
            return 1;
        }

        var model = settings.Model;
        if (string.IsNullOrWhiteSpace(model))
        {
            model = Config.Get("defaults.model", string.Empty);
        }

        if (string.IsNullOrWhiteSpace(model) && string.Equals(backendName, "opencode", StringComparison.OrdinalIgnoreCase))
        {
            model = Config.Get("opencode.default_model", string.Empty);
        }

        if (string.IsNullOrWhiteSpace(model))
        {
            model = backend.DefaultModel ?? string.Empty;
        }

        var stack = StackDetector.Detect(targetDir);

        if (interactive && stack.StackIds.Count > 0)
        {
            Console.Error.WriteLine("Step 5/6: Stack detection");
            if (stack.StackIds.Count > 0)
            {
                Console.Error.WriteLine("Detected stacks:");
                for (var i = 0; i < stack.StackIds.Count; i++)
                {
                    Console.Error.WriteLine($"  {i + 1}) {stack.StackIds[i]}");
                }
            }
            else
            {
                Console.Error.WriteLine("No stack files detected.");
            }
        }

        if (interactive && stack.StackIds.Count > 1)
        {
            var confirm = PromptInput("Use all detected stacks? (Y/n)", "Y");
            if (string.Equals(confirm, "n", StringComparison.OrdinalIgnoreCase))
            {
                var selection = PromptInput("Select stacks by number (comma-separated, press Enter for all)", string.Empty);
                stack.SelectStacks(selection);
            }
        }

        var stackSummaryText = stack.FormatSummary("##");
        var detectedStackList = stack.StackIds.Count > 0
            ? string.Join(", ", stack.StackIds)
            : "None detected";

        var configContextFiles = Config.Get("defaults.context_files", string.Empty);
        var contextFiles = ContextFileListBuilder.Build(targetDir, settings.Context, configContextFiles);
        var contextSection = contextFiles.Count > 0
            ? string.Join(Environment.NewLine, contextFiles)
            : "None.";

        var sources = SourceListBuilder.BuildSources(sourcesInput, stack);
        var sourcesOrigin = sources.Origin;
        var sourcesSection = sources.List.Count > 0
            ? string.Join(Environment.NewLine, sources.List)
            : "None.";
        var warningsSection = sources.List.Count == 0
            ? "No reliable external sources were provided or discovered. Verify requirements and stack assumptions before implementation."
            : string.Empty;

        if (interactive)
        {
            Console.Error.WriteLine("Step 6/6: Review summary");
        }
        else
        {
            Console.Error.WriteLine("Summary");
        }

        Console.Error.WriteLine("Summary:");
        Console.Error.WriteLine($"  Goal: {goal}");
        Console.Error.WriteLine($"  Constraints: {constraints}");
        Console.Error.WriteLine($"  Output: {outputPath}");
        Console.Error.WriteLine($"  Detected stacks: {detectedStackList}");
        Console.Error.WriteLine($"  Sources: {sourcesOrigin}");
        Console.Error.WriteLine($"  Allow missing context: {settings.AllowMissingContext}");
        Console.Error.WriteLine($"  Backend: {backendName}");
        if (!string.IsNullOrWhiteSpace(model))
        {
            Console.Error.WriteLine($"  Model: {model}");
        }
        Console.Error.WriteLine("  Context files:");
        if (contextFiles.Count > 0)
        {
            foreach (var line in contextFiles)
            {
                Console.Error.WriteLine($"    - {line}");
            }
        }
        else
        {
            Console.Error.WriteLine("    - None");
        }

        if (interactive)
        {
            var proceed = PromptInput("Proceed to generate PRD? (y/N)", "N");
            if (!string.Equals(proceed, "y", StringComparison.OrdinalIgnoreCase))
            {
                Console.Error.WriteLine("Error: PRD generation cancelled.");
                return 1;
            }
        }
        else
        {
            Console.Error.WriteLine("Non-interactive mode: skipping confirmation.");
        }

        var templateText = TemplateLoader.Load(targetDir);
        var prompt = PromptBuilder.BuildPrompt(targetDir, goal, constraints, stackSummaryText, sourcesSection, warningsSection, contextSection, templateText);

        var rawOutputFile = Path.GetTempFileName();
        var generatedPrdFile = Path.GetTempFileName();
        var errorMessages = new List<string>();
        var keepRawOutput = false;

        try
        {
            var exitCode = await backend.RunIterationAsync(prompt, string.IsNullOrWhiteSpace(model) ? null : model, rawOutputFile, cancellationToken);
            if (exitCode != 0)
            {
                Console.Error.WriteLine($"Warning: PRD generation failed (backend exit code {exitCode}).");
                Console.Error.WriteLine($"Raw backend output saved to: {rawOutputFile}");
                keepRawOutput = true;
                return 1;
            }

            var result = backend.ParseText(rawOutputFile);
            if (string.IsNullOrWhiteSpace(result))
            {
                Console.Error.WriteLine("Warning: PRD generation returned empty output.");
                Console.Error.WriteLine($"Raw backend output saved to: {rawOutputFile}");
                keepRawOutput = true;
                return 1;
            }

            File.WriteAllText(generatedPrdFile, result, new UTF8Encoding(false));
            PrdSanitizer.SanitizeGeneratedFile(generatedPrdFile, targetDir, contextFiles);

            if (!PrdValidator.Validate(generatedPrdFile, targetDir, errorMessages.Add, settings.AllowMissingContext))
            {
                Console.Error.WriteLine("Warning: Generated PRD failed validation.");
                foreach (var error in errorMessages)
                {
                    Console.Error.WriteLine(error);
                }

                var invalidPath = outputPath;
                if (!settings.Force)
                {
                    invalidPath = outputPath.EndsWith(".md", StringComparison.OrdinalIgnoreCase)
                        ? outputPath[..^3] + ".invalid.md"
                        : outputPath + ".invalid";
                }

                Directory.CreateDirectory(Path.GetDirectoryName(invalidPath) ?? targetDir);
                File.Move(generatedPrdFile, invalidPath, true);
                Console.Error.WriteLine($"Warning: Saved invalid PRD to: {invalidPath}");
                return 1;
            }

            Directory.CreateDirectory(Path.GetDirectoryName(outputPath) ?? targetDir);
            File.Move(generatedPrdFile, outputPath, true);
            Console.WriteLine($"PRD created: {outputPath}");

            var relativeOutput = outputPath.StartsWith(targetDir + Path.DirectorySeparatorChar, StringComparison.Ordinal)
                ? outputPath[(targetDir.Length + 1)..]
                : outputPath;

            Console.WriteLine("Next step:");
            var modelSuffix = string.IsNullOrWhiteSpace(model) ? string.Empty : $" --model {model}";
            Console.WriteLine($"  gralph start {targetDir} --task-file {relativeOutput} --no-tmux --backend {backendName}{modelSuffix} --strict-prd");
            return 0;
        }
        finally
        {
            if (!keepRawOutput && File.Exists(rawOutputFile))
            {
                File.Delete(rawOutputFile);
            }

            if (File.Exists(generatedPrdFile))
            {
                File.Delete(generatedPrdFile);
            }
        }
    }

    private static string ResolveTargetDir(string? dir)
    {
        if (string.IsNullOrWhiteSpace(dir))
        {
            return Directory.GetCurrentDirectory();
        }

        return Path.GetFullPath(dir);
    }

    private static bool ResolveInteractive(bool? interactive)
    {
        if (interactive.HasValue)
        {
            return interactive.Value;
        }

        return !Console.IsInputRedirected && !Console.IsOutputRedirected;
    }

    private static string PromptInput(string prompt, string defaultValue)
    {
        if (!string.IsNullOrWhiteSpace(defaultValue))
        {
            Console.Error.Write($"{prompt} [{defaultValue}]: ");
        }
        else
        {
            Console.Error.Write($"{prompt}: ");
        }

        var input = Console.ReadLine();
        if (string.IsNullOrWhiteSpace(input))
        {
            return defaultValue;
        }

        return input.Trim();
    }

    private static string PromptMultiline(string prompt)
    {
        Console.Error.WriteLine($"{prompt} (finish with empty line):");
        var builder = new StringBuilder();
        while (true)
        {
            var line = Console.ReadLine();
            if (line is null || line.Length == 0)
            {
                break;
            }

            if (builder.Length > 0)
            {
                builder.AppendLine();
            }
            builder.Append(line);
        }

        return builder.ToString();
    }
}

internal sealed class StackDetectionResult
{
    public string RootDir { get; set; } = string.Empty;
    public List<string> StackIds { get; } = new();
    public List<string> Languages { get; } = new();
    public List<string> Frameworks { get; } = new();
    public List<string> Tools { get; } = new();
    public List<string> Runtimes { get; } = new();
    public List<string> PackageManagers { get; } = new();
    public List<string> Evidence { get; } = new();
    public List<string> SelectedStackIds { get; } = new();

    public void SelectStacks(string selection)
    {
        SelectedStackIds.Clear();
        if (string.IsNullOrWhiteSpace(selection))
        {
            SelectedStackIds.AddRange(StackIds);
            return;
        }

        var indexes = selection.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        foreach (var index in indexes)
        {
            if (int.TryParse(index, out var parsed) && parsed >= 1 && parsed <= StackIds.Count)
            {
                SelectedStackIds.Add(StackIds[parsed - 1]);
            }
        }

        if (SelectedStackIds.Count == 0)
        {
            SelectedStackIds.AddRange(StackIds);
        }
    }

    public string FormatSummary(string headerPrefix)
    {
        var builder = new StringBuilder();
        builder.AppendLine($"{headerPrefix} Stack Summary");
        builder.AppendLine();
        builder.AppendLine($"- Stacks: {JoinOrFallback(StackIds, "Unknown")}");
        builder.AppendLine($"- Languages: {JoinOrFallback(Languages, "Unknown")}");
        builder.AppendLine($"- Runtimes: {JoinOrFallback(Runtimes, "Unknown")}");
        builder.AppendLine($"- Frameworks: {JoinOrFallback(Frameworks, "None detected")}");
        builder.AppendLine($"- Tools: {JoinOrFallback(Tools, "None detected")}");
        builder.AppendLine($"- Package managers: {JoinOrFallback(PackageManagers, "None detected")}");

        if (SelectedStackIds.Count > 0 && SelectedStackIds.Count < StackIds.Count)
        {
            builder.AppendLine($"- Stack focus: {string.Join(", ", SelectedStackIds)}");
        }

        builder.AppendLine();
        builder.AppendLine("Evidence:");
        if (Evidence.Count > 0)
        {
            foreach (var item in Evidence)
            {
                builder.AppendLine($"- {item}");
            }
        }
        else
        {
            builder.AppendLine("- None found");
        }

        return builder.ToString().TrimEnd();
    }

    private static string JoinOrFallback(List<string> values, string fallback)
    {
        return values.Count > 0 ? string.Join(", ", values) : fallback;
    }
}

internal static class StackDetector
{
    private static readonly Regex PoetryRegex = new("\\[tool\\.poetry\\]", RegexOptions.Compiled | RegexOptions.IgnoreCase);

    public static StackDetectionResult Detect(string targetDir)
    {
        var result = new StackDetectionResult();
        if (string.IsNullOrWhiteSpace(targetDir) || !Directory.Exists(targetDir))
        {
            return result;
        }

        result.RootDir = targetDir;

        DetectNodeStack(targetDir, result);
        DetectGoStack(targetDir, result);
        DetectRustStack(targetDir, result);
        DetectPythonStack(targetDir, result);
        DetectRubyStack(targetDir, result);
        DetectElixirStack(targetDir, result);
        DetectPhpStack(targetDir, result);
        DetectJavaStack(targetDir, result);
        DetectDotNetStack(targetDir, result);
        DetectTooling(targetDir, result);

        result.SelectedStackIds.AddRange(result.StackIds);
        return result;
    }

    private static void DetectNodeStack(string targetDir, StackDetectionResult result)
    {
        var packageJson = Path.Combine(targetDir, "package.json");
        if (!File.Exists(packageJson))
        {
            return;
        }

        AddUnique(result.StackIds, "Node.js");
        AddUnique(result.Runtimes, "Node.js");
        AddUnique(result.Languages, "JavaScript");
        AddEvidence(result, packageJson);

        var tsconfig = Path.Combine(targetDir, "tsconfig.json");
        if (File.Exists(tsconfig))
        {
            AddUnique(result.Languages, "TypeScript");
            AddEvidence(result, tsconfig);
        }

        AddPackageManagerIfExists(targetDir, "pnpm-lock.yaml", "pnpm", result);
        AddPackageManagerIfExists(targetDir, "yarn.lock", "yarn", result);
        AddPackageManagerIfExists(targetDir, "package-lock.json", "npm", result);
        AddPackageManagerIfExists(targetDir, "bun.lockb", "bun", result, runtime: "Bun");
        AddPackageManagerIfExists(targetDir, "bunfig.toml", "bun", result, runtime: "Bun");

        AddFrameworkIfExists(targetDir, "next.config.js", "Next.js", result);
        AddFrameworkIfExists(targetDir, "next.config.mjs", "Next.js", result);
        AddFrameworkIfExists(targetDir, "next.config.cjs", "Next.js", result);
        AddFrameworkIfExists(targetDir, "nuxt.config.js", "Nuxt", result);
        AddFrameworkIfExists(targetDir, "nuxt.config.ts", "Nuxt", result);
        AddFrameworkIfExists(targetDir, "svelte.config.js", "Svelte", result);
        AddFrameworkIfExists(targetDir, "svelte.config.ts", "Svelte", result);
        AddToolIfExists(targetDir, "vite.config.js", "Vite", result);
        AddToolIfExists(targetDir, "vite.config.ts", "Vite", result);
        AddToolIfExists(targetDir, "vite.config.mjs", "Vite", result);
        AddFrameworkIfExists(targetDir, "angular.json", "Angular", result);
        AddFrameworkIfExists(targetDir, "vue.config.js", "Vue", result);

        if (PackageJsonHasDependency(packageJson, "react"))
        {
            AddUnique(result.Frameworks, "React");
        }
        if (PackageJsonHasDependency(packageJson, "next"))
        {
            AddUnique(result.Frameworks, "Next.js");
        }
        if (PackageJsonHasDependency(packageJson, "vue"))
        {
            AddUnique(result.Frameworks, "Vue");
        }
        if (PackageJsonHasDependency(packageJson, "@angular/core"))
        {
            AddUnique(result.Frameworks, "Angular");
        }
        if (PackageJsonHasDependency(packageJson, "svelte"))
        {
            AddUnique(result.Frameworks, "Svelte");
        }
        if (PackageJsonHasDependency(packageJson, "nuxt"))
        {
            AddUnique(result.Frameworks, "Nuxt");
        }
        if (PackageJsonHasDependency(packageJson, "express"))
        {
            AddUnique(result.Frameworks, "Express");
        }
        if (PackageJsonHasDependency(packageJson, "fastify"))
        {
            AddUnique(result.Frameworks, "Fastify");
        }
        if (PackageJsonHasDependency(packageJson, "@nestjs/core"))
        {
            AddUnique(result.Frameworks, "NestJS");
        }
    }

    private static void DetectGoStack(string targetDir, StackDetectionResult result)
    {
        var goMod = Path.Combine(targetDir, "go.mod");
        if (!File.Exists(goMod))
        {
            return;
        }

        AddUnique(result.StackIds, "Go");
        AddUnique(result.Languages, "Go");
        AddUnique(result.Tools, "Go modules");
        AddEvidence(result, goMod);
    }

    private static void DetectRustStack(string targetDir, StackDetectionResult result)
    {
        var cargo = Path.Combine(targetDir, "Cargo.toml");
        if (!File.Exists(cargo))
        {
            return;
        }

        AddUnique(result.StackIds, "Rust");
        AddUnique(result.Languages, "Rust");
        AddUnique(result.Tools, "Cargo");
        AddEvidence(result, cargo);
    }

    private static void DetectPythonStack(string targetDir, StackDetectionResult result)
    {
        var pyproject = Path.Combine(targetDir, "pyproject.toml");
        var requirements = Path.Combine(targetDir, "requirements.txt");
        var poetryLock = Path.Combine(targetDir, "poetry.lock");
        var pipfile = Path.Combine(targetDir, "Pipfile");
        var pipfileLock = Path.Combine(targetDir, "Pipfile.lock");

        if (!File.Exists(pyproject) && !File.Exists(requirements) && !File.Exists(poetryLock) && !File.Exists(pipfile) && !File.Exists(pipfileLock))
        {
            return;
        }

        AddUnique(result.StackIds, "Python");
        AddUnique(result.Languages, "Python");

        if (File.Exists(pyproject))
        {
            AddEvidence(result, pyproject);
            var content = File.ReadAllText(pyproject);
            if (PoetryRegex.IsMatch(content))
            {
                AddUnique(result.Tools, "Poetry");
            }

            DetectPythonFrameworks(content, result);
        }

        if (File.Exists(requirements))
        {
            AddEvidence(result, requirements);
            var content = File.ReadAllText(requirements);
            DetectPythonFrameworks(content, result);
        }

        if (File.Exists(poetryLock))
        {
            AddEvidence(result, poetryLock);
        }

        if (File.Exists(pipfile))
        {
            AddEvidence(result, pipfile);
        }

        if (File.Exists(pipfileLock))
        {
            AddEvidence(result, pipfileLock);
        }
    }

    private static void DetectRubyStack(string targetDir, StackDetectionResult result)
    {
        var gemfile = Path.Combine(targetDir, "Gemfile");
        if (!File.Exists(gemfile))
        {
            return;
        }

        AddUnique(result.StackIds, "Ruby");
        AddUnique(result.Languages, "Ruby");
        AddEvidence(result, gemfile);

        var content = File.ReadAllText(gemfile);
        if (content.Contains("rails", StringComparison.OrdinalIgnoreCase))
        {
            AddUnique(result.Frameworks, "Rails");
        }
        if (content.Contains("sinatra", StringComparison.OrdinalIgnoreCase))
        {
            AddUnique(result.Frameworks, "Sinatra");
        }
    }

    private static void DetectElixirStack(string targetDir, StackDetectionResult result)
    {
        var mix = Path.Combine(targetDir, "mix.exs");
        if (!File.Exists(mix))
        {
            return;
        }

        AddUnique(result.StackIds, "Elixir");
        AddUnique(result.Languages, "Elixir");
        AddEvidence(result, mix);

        var content = File.ReadAllText(mix);
        if (content.Contains("phoenix", StringComparison.OrdinalIgnoreCase))
        {
            AddUnique(result.Frameworks, "Phoenix");
        }
    }

    private static void DetectPhpStack(string targetDir, StackDetectionResult result)
    {
        var composer = Path.Combine(targetDir, "composer.json");
        if (!File.Exists(composer))
        {
            return;
        }

        AddUnique(result.StackIds, "PHP");
        AddUnique(result.Languages, "PHP");
        AddEvidence(result, composer);

        var content = File.ReadAllText(composer);
        if (content.Contains("laravel", StringComparison.OrdinalIgnoreCase))
        {
            AddUnique(result.Frameworks, "Laravel");
        }
    }

    private static void DetectJavaStack(string targetDir, StackDetectionResult result)
    {
        var pom = Path.Combine(targetDir, "pom.xml");
        if (File.Exists(pom))
        {
            AddUnique(result.StackIds, "Java");
            AddUnique(result.Languages, "Java");
            AddUnique(result.Tools, "Maven");
            AddEvidence(result, pom);
            var content = File.ReadAllText(pom);
            if (content.Contains("spring-boot", StringComparison.OrdinalIgnoreCase))
            {
                AddUnique(result.Frameworks, "Spring Boot");
            }
        }

        var gradle = Path.Combine(targetDir, "build.gradle");
        if (File.Exists(gradle))
        {
            AddUnique(result.StackIds, "Java");
            AddUnique(result.Languages, "Java");
            AddUnique(result.Tools, "Gradle");
            AddEvidence(result, gradle);
            var content = File.ReadAllText(gradle);
            if (content.Contains("spring-boot", StringComparison.OrdinalIgnoreCase))
            {
                AddUnique(result.Frameworks, "Spring Boot");
            }
        }

        var gradleKts = Path.Combine(targetDir, "build.gradle.kts");
        if (File.Exists(gradleKts))
        {
            AddUnique(result.StackIds, "Java");
            AddUnique(result.Languages, "Java");
            AddUnique(result.Tools, "Gradle");
            AddEvidence(result, gradleKts);
            var content = File.ReadAllText(gradleKts);
            if (content.Contains("spring-boot", StringComparison.OrdinalIgnoreCase))
            {
                AddUnique(result.Frameworks, "Spring Boot");
            }
        }
    }

    private static void DetectDotNetStack(string targetDir, StackDetectionResult result)
    {
        var csproj = Directory.GetFiles(targetDir, "*.csproj", SearchOption.TopDirectoryOnly);
        var sln = Directory.GetFiles(targetDir, "*.sln", SearchOption.TopDirectoryOnly);
        if (csproj.Length == 0 && sln.Length == 0)
        {
            return;
        }

        AddUnique(result.StackIds, ".NET");
        AddUnique(result.Languages, "C#");

        foreach (var file in csproj)
        {
            AddEvidence(result, file);
        }

        foreach (var file in sln)
        {
            AddEvidence(result, file);
        }
    }

    private static void DetectTooling(string targetDir, StackDetectionResult result)
    {
        AddToolIfExists(targetDir, "Dockerfile", "Docker", result);
        AddToolIfExists(targetDir, "docker-compose.yml", "Docker Compose", result);
        AddToolIfExists(targetDir, "docker-compose.yaml", "Docker Compose", result);
        AddToolIfExists(targetDir, "Makefile", "Make", result);

        var terraformFiles = Directory.GetFiles(targetDir, "*.tf", SearchOption.TopDirectoryOnly);
        if (terraformFiles.Length > 0)
        {
            AddUnique(result.Tools, "Terraform");
            foreach (var file in terraformFiles)
            {
                AddEvidence(result, file);
            }
        }
    }

    private static void DetectPythonFrameworks(string content, StackDetectionResult result)
    {
        if (content.Contains("django", StringComparison.OrdinalIgnoreCase))
        {
            AddUnique(result.Frameworks, "Django");
        }
        if (content.Contains("flask", StringComparison.OrdinalIgnoreCase))
        {
            AddUnique(result.Frameworks, "Flask");
        }
        if (content.Contains("fastapi", StringComparison.OrdinalIgnoreCase))
        {
            AddUnique(result.Frameworks, "FastAPI");
        }
    }

    private static void AddPackageManagerIfExists(string targetDir, string fileName, string name, StackDetectionResult result, string? runtime = null)
    {
        var path = Path.Combine(targetDir, fileName);
        if (!File.Exists(path))
        {
            return;
        }

        AddUnique(result.PackageManagers, name);
        if (!string.IsNullOrWhiteSpace(runtime))
        {
            AddUnique(result.Runtimes, runtime);
        }
        AddEvidence(result, path);
    }

    private static void AddFrameworkIfExists(string targetDir, string fileName, string name, StackDetectionResult result)
    {
        var path = Path.Combine(targetDir, fileName);
        if (File.Exists(path))
        {
            AddUnique(result.Frameworks, name);
            AddEvidence(result, path);
        }
    }

    private static void AddToolIfExists(string targetDir, string fileName, string name, StackDetectionResult result)
    {
        var path = Path.Combine(targetDir, fileName);
        if (File.Exists(path))
        {
            AddUnique(result.Tools, name);
            AddEvidence(result, path);
        }
    }

    private static bool PackageJsonHasDependency(string packageJson, string dependency)
    {
        try
        {
            var json = File.ReadAllText(packageJson);
            using var doc = JsonDocument.Parse(json);
            return DependencyExists(doc.RootElement, dependency);
        }
        catch (JsonException)
        {
            return File.ReadAllText(packageJson).Contains($"\"{dependency}\"", StringComparison.OrdinalIgnoreCase);
        }
        catch (IOException)
        {
            return false;
        }
    }

    private static bool DependencyExists(JsonElement root, string dependency)
    {
        return TryHasDependency(root, "dependencies", dependency)
               || TryHasDependency(root, "devDependencies", dependency)
               || TryHasDependency(root, "peerDependencies", dependency);
    }

    private static bool TryHasDependency(JsonElement root, string section, string dependency)
    {
        if (!root.TryGetProperty(section, out var deps) || deps.ValueKind != JsonValueKind.Object)
        {
            return false;
        }

        return deps.TryGetProperty(dependency, out _);
    }

    private static void AddUnique(ICollection<string> list, string value)
    {
        if (!list.Contains(value))
        {
            list.Add(value);
        }
    }

    private static void AddEvidence(StackDetectionResult result, string path)
    {
        var display = path;
        var root = result.RootDir;
        if (!string.IsNullOrWhiteSpace(root)
            && path.StartsWith(root + Path.DirectorySeparatorChar, StringComparison.Ordinal))
        {
            display = path[(root.Length + 1)..];
        }

        if (!result.Evidence.Contains(display))
        {
            result.Evidence.Add(display);
        }
    }
}

internal static class ContextFileListBuilder
{
    private static readonly string[] DefaultEntries = new[]
    {
        "README.md",
        "ARCHITECTURE.md",
        "DECISIONS.md",
        "CHANGELOG.md",
        "RISK_REGISTER.md",
        "PROCESS.md",
        "PRD.template.md",
        "bin/gralph",
        "config/default.yaml",
        "opencode.json",
        "completions/gralph.bash",
        "completions/gralph.zsh"
    };

    public static List<string> Build(string targetDir, string? userList, string? configList)
    {
        var entries = new List<string>();
        var seen = new HashSet<string>(StringComparer.Ordinal);

        void AddEntry(string? item)
        {
            if (string.IsNullOrWhiteSpace(item))
            {
                return;
            }

            var candidate = item.Trim();
            var resolved = Path.IsPathRooted(candidate)
                ? candidate
                : Path.Combine(targetDir, candidate);

            if (!File.Exists(resolved))
            {
                return;
            }

            var display = resolved.StartsWith(targetDir + Path.DirectorySeparatorChar, StringComparison.Ordinal)
                ? resolved[(targetDir.Length + 1)..]
                : resolved;

            if (seen.Add(display))
            {
                entries.Add(display);
            }
        }

        foreach (var item in ParseCsvList(configList))
        {
            AddEntry(item);
        }

        foreach (var item in ParseCsvList(userList))
        {
            AddEntry(item);
        }

        foreach (var entry in DefaultEntries)
        {
            AddEntry(entry);
        }

        AddGlobEntries(targetDir, "lib", "*.sh", entries, seen);
        AddGlobEntries(targetDir, Path.Combine("lib", "backends"), "*.sh", entries, seen);
        AddGlobEntries(targetDir, "tests", "*.sh", entries, seen);

        return entries;
    }

    private static void AddGlobEntries(string targetDir, string subDir, string pattern, List<string> entries, HashSet<string> seen)
    {
        var root = Path.Combine(targetDir, subDir);
        if (!Directory.Exists(root))
        {
            return;
        }

        foreach (var file in Directory.GetFiles(root, pattern, SearchOption.TopDirectoryOnly))
        {
            var display = file.StartsWith(targetDir + Path.DirectorySeparatorChar, StringComparison.Ordinal)
                ? file[(targetDir.Length + 1)..]
                : file;
            if (seen.Add(display))
            {
                entries.Add(display);
            }
        }
    }

    private static IEnumerable<string> ParseCsvList(string? raw)
    {
        if (string.IsNullOrWhiteSpace(raw))
        {
            yield break;
        }

        var parts = raw.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        foreach (var part in parts)
        {
            if (!string.IsNullOrWhiteSpace(part))
            {
                yield return part;
            }
        }
    }
}

internal sealed class SourceListResult
{
    public List<string> List { get; } = new();
    public string Origin { get; set; } = "none";
}

internal static class SourceListBuilder
{
    public static SourceListResult BuildSources(string? userSources, StackDetectionResult stack)
    {
        var result = new SourceListResult();
        var normalized = NormalizeCsvList(userSources);
        if (normalized.Count > 0)
        {
            result.Origin = "user";
            result.List.AddRange(Deduplicate(normalized));
            return result;
        }

        var official = CollectOfficialSources(stack);
        if (official.Count > 0)
        {
            result.Origin = "official";
            result.List.AddRange(Deduplicate(official));
            return result;
        }

        result.Origin = "none";
        return result;
    }

    private static List<string> NormalizeCsvList(string? raw)
    {
        var items = new List<string>();
        if (string.IsNullOrWhiteSpace(raw))
        {
            return items;
        }

        foreach (var entry in raw.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            if (!string.IsNullOrWhiteSpace(entry))
            {
                items.Add(entry);
            }
        }

        return items;
    }

    private static List<string> Deduplicate(List<string> input)
    {
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var output = new List<string>();
        foreach (var item in input)
        {
            if (seen.Add(item))
            {
                output.Add(item);
            }
        }

        return output;
    }

    private static List<string> CollectOfficialSources(StackDetectionResult stack)
    {
        var sources = new List<string>();
        foreach (var item in stack.SelectedStackIds)
        {
            switch (item)
            {
                case "Node.js":
                    sources.Add("https://nodejs.org/docs/latest/api/");
                    break;
                case "Go":
                    sources.Add("https://go.dev/doc/");
                    break;
                case "Rust":
                    sources.Add("https://doc.rust-lang.org/");
                    break;
                case "Python":
                    sources.Add("https://docs.python.org/3/");
                    break;
                case "Ruby":
                    sources.Add("https://www.ruby-lang.org/en/documentation/");
                    break;
                case "Java":
                    sources.Add("https://docs.oracle.com/en/java/");
                    break;
                case ".NET":
                    sources.Add("https://learn.microsoft.com/dotnet/");
                    break;
                case "PHP":
                    sources.Add("https://www.php.net/manual/en/");
                    break;
                case "Elixir":
                    sources.Add("https://elixir-lang.org/docs.html");
                    break;
            }
        }

        foreach (var item in stack.Frameworks)
        {
            switch (item)
            {
                case "React":
                    sources.Add("https://react.dev/");
                    break;
                case "Next.js":
                    sources.Add("https://nextjs.org/docs");
                    break;
                case "Vue":
                    sources.Add("https://vuejs.org/guide/");
                    break;
                case "Angular":
                    sources.Add("https://angular.dev/guide");
                    break;
                case "Svelte":
                    sources.Add("https://svelte.dev/docs");
                    break;
                case "Nuxt":
                    sources.Add("https://nuxt.com/docs");
                    break;
                case "Express":
                    sources.Add("https://expressjs.com/");
                    break;
                case "Fastify":
                    sources.Add("https://www.fastify.io/docs/latest/");
                    break;
                case "NestJS":
                    sources.Add("https://docs.nestjs.com/");
                    break;
                case "Django":
                    sources.Add("https://docs.djangoproject.com/en/stable/");
                    break;
                case "Flask":
                    sources.Add("https://flask.palletsprojects.com/");
                    break;
                case "FastAPI":
                    sources.Add("https://fastapi.tiangolo.com/");
                    break;
                case "Rails":
                    sources.Add("https://guides.rubyonrails.org/");
                    break;
                case "Sinatra":
                    sources.Add("https://sinatrarb.com/documentation.html");
                    break;
                case "Phoenix":
                    sources.Add("https://hexdocs.pm/phoenix/");
                    break;
                case "Laravel":
                    sources.Add("https://laravel.com/docs");
                    break;
                case "Spring Boot":
                    sources.Add("https://docs.spring.io/spring-boot/docs/current/reference/html/");
                    break;
            }
        }

        foreach (var item in stack.Tools)
        {
            switch (item)
            {
                case "Vite":
                    sources.Add("https://vitejs.dev/guide/");
                    break;
                case "Docker":
                    sources.Add("https://docs.docker.com/");
                    break;
                case "Docker Compose":
                    sources.Add("https://docs.docker.com/compose/");
                    break;
                case "Make":
                    sources.Add("https://www.gnu.org/software/make/manual/make.html");
                    break;
                case "Terraform":
                    sources.Add("https://developer.hashicorp.com/terraform/docs");
                    break;
                case "Go modules":
                    sources.Add("https://go.dev/ref/mod");
                    break;
                case "Cargo":
                    sources.Add("https://doc.rust-lang.org/cargo/");
                    break;
                case "Maven":
                    sources.Add("https://maven.apache.org/guides/");
                    break;
                case "Gradle":
                    sources.Add("https://docs.gradle.org/current/userguide/userguide.html");
                    break;
                case "Poetry":
                    sources.Add("https://python-poetry.org/docs/");
                    break;
            }
        }

        foreach (var item in stack.PackageManagers)
        {
            switch (item)
            {
                case "pnpm":
                    sources.Add("https://pnpm.io/");
                    break;
                case "yarn":
                    sources.Add("https://yarnpkg.com/");
                    break;
                case "npm":
                    sources.Add("https://docs.npmjs.com/");
                    break;
                case "bun":
                    sources.Add("https://bun.sh/docs");
                    break;
            }
        }

        return sources;
    }
}

internal static class TemplateLoader
{
    public static string Load(string targetDir)
    {
        var candidate = Path.Combine(targetDir, "PRD.template.md");
        if (File.Exists(candidate))
        {
            return File.ReadAllText(candidate);
        }

        var fallback = Path.Combine(Directory.GetCurrentDirectory(), "PRD.template.md");
        if (File.Exists(fallback))
        {
            return File.ReadAllText(fallback);
        }

        return """
## Overview

Briefly describe the project, goals, and intended users.

## Problem Statement

- What problem does this solve?
- What pain points exist today?

## Solution

High-level solution summary.

---

## Functional Requirements

### FR-1: Core Feature

Describe the primary user-facing behavior.

### FR-2: Secondary Feature

Describe supporting behavior.

---

## Non-Functional Requirements

### NFR-1: Performance

- Example: Response times under 200ms for key operations.

### NFR-2: Reliability

- Example: Crash recovery or retries where appropriate.

---

## Implementation Tasks

Each task must use a `### Task <ID>` block header and include the required fields.
Each task block must contain exactly one unchecked task line.

### Task EX-1

- **ID** EX-1
- **Context Bundle** `path/to/file`, `path/to/other`
- **DoD** Define the done criteria for this task.
- **Checklist**
  * First verification item.
  * Second verification item.
- **Dependencies** None
- [ ] EX-1 Short task summary

---

## Success Criteria

- Define measurable outcomes that indicate completion.

---

## Sources

- List authoritative URLs used as source of truth.

---

## Warnings

- Only include this section if no reliable sources were found.
- State what is missing and what must be verified.
""";
    }
}

internal static class PromptBuilder
{
    public static string BuildPrompt(
        string targetDir,
        string goal,
        string constraints,
        string stackSummary,
        string sources,
        string warnings,
        string contextSection,
        string templateText)
    {
        var stackSummaryText = string.IsNullOrWhiteSpace(stackSummary) ? "None detected." : stackSummary;
        var warningsText = string.IsNullOrWhiteSpace(warnings) ? "None." : warnings;

        return $"""
You are generating a gralph PRD in markdown. The output must be spec-compliant and grounded in the repository.

Project directory: {targetDir}

Goal:
{goal}

Constraints:
{constraints}

Detected stack summary (from repository files):
{stackSummaryText}

Sources (authoritative URLs or references):
{sources}

Warnings (only include in the PRD if Sources is empty):
{warningsText}

Context files (read these first if present):
{contextSection}

Requirements:
- Output only the PRD markdown with no commentary or code fences.
- Use ASCII only.
- Do not include an "Open Questions" section.
- Do not use any checkboxes outside task blocks.
- Context Bundle entries must be real files in the repo and must be selected from the Context files list above.
- If a task creates new files, do not list the new files in Context Bundle; cite the closest existing files instead.
- Use atomic, granular tasks grounded in the repo and context files.
- Each task block must use a '### Task <ID>' header and include **ID**, **Context Bundle**, **DoD**, **Checklist**, **Dependencies**.
- Each task block must contain exactly one unchecked task line like '- [ ] <ID> <summary>'.
- If Sources is empty, include a 'Warnings' section with the warning text above and no checkboxes.
- Do not invent stack, frameworks, or files not supported by the context files and stack summary.

Template:
{templateText}
""";
    }
}
