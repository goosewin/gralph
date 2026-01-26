using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;

namespace Gralph.Prd;

public static class PrdStackDetector
{
    public static PrdStackSummary Detect(string targetDir)
    {
        var stackIds = new HashSet<string>(StringComparer.Ordinal);
        var languages = new HashSet<string>(StringComparer.Ordinal);
        var frameworks = new HashSet<string>(StringComparer.Ordinal);
        var tools = new HashSet<string>(StringComparer.Ordinal);
        var runtimes = new HashSet<string>(StringComparer.Ordinal);
        var packageManagers = new HashSet<string>(StringComparer.Ordinal);
        var evidence = new List<string>();

        if (string.IsNullOrWhiteSpace(targetDir) || !Directory.Exists(targetDir))
        {
            return BuildSummary(stackIds, languages, frameworks, tools, runtimes, packageManagers, evidence);
        }

        var root = Path.GetFullPath(targetDir);

        void RecordEvidence(string path)
        {
            if (string.IsNullOrWhiteSpace(path))
            {
                return;
            }

            var full = Path.GetFullPath(path);
            var display = full.StartsWith(root, StringComparison.Ordinal)
                ? Path.GetRelativePath(root, full)
                : full;
            if (!evidence.Contains(display, StringComparer.Ordinal))
            {
                evidence.Add(display);
            }
        }

        var packageJson = Path.Combine(root, "package.json");
        if (File.Exists(packageJson))
        {
            stackIds.Add("Node.js");
            runtimes.Add("Node.js");
            languages.Add("JavaScript");
            RecordEvidence(packageJson);

            var tsconfig = Path.Combine(root, "tsconfig.json");
            if (File.Exists(tsconfig))
            {
                languages.Add("TypeScript");
                RecordEvidence(tsconfig);
            }

            AddIfExists(root, "pnpm-lock.yaml", packageManagers, "pnpm", RecordEvidence);
            AddIfExists(root, "yarn.lock", packageManagers, "yarn", RecordEvidence);
            AddIfExists(root, "package-lock.json", packageManagers, "npm", RecordEvidence);

            if (File.Exists(Path.Combine(root, "bun.lockb")) || File.Exists(Path.Combine(root, "bunfig.toml")))
            {
                runtimes.Add("Bun");
                packageManagers.Add("bun");
                if (File.Exists(Path.Combine(root, "bun.lockb")))
                {
                    RecordEvidence(Path.Combine(root, "bun.lockb"));
                }
                if (File.Exists(Path.Combine(root, "bunfig.toml")))
                {
                    RecordEvidence(Path.Combine(root, "bunfig.toml"));
                }
            }

            AddIfExists(root, "next.config.js", frameworks, "Next.js", RecordEvidence);
            AddIfExists(root, "next.config.mjs", frameworks, "Next.js", RecordEvidence);
            AddIfExists(root, "next.config.cjs", frameworks, "Next.js", RecordEvidence);
            AddIfExists(root, "nuxt.config.js", frameworks, "Nuxt", RecordEvidence);
            AddIfExists(root, "nuxt.config.ts", frameworks, "Nuxt", RecordEvidence);
            AddIfExists(root, "svelte.config.js", frameworks, "Svelte", RecordEvidence);
            AddIfExists(root, "svelte.config.ts", frameworks, "Svelte", RecordEvidence);
            AddIfExists(root, "vite.config.js", tools, "Vite", RecordEvidence);
            AddIfExists(root, "vite.config.ts", tools, "Vite", RecordEvidence);
            AddIfExists(root, "vite.config.mjs", tools, "Vite", RecordEvidence);
            AddIfExists(root, "angular.json", frameworks, "Angular", RecordEvidence);
            AddIfExists(root, "vue.config.js", frameworks, "Vue", RecordEvidence);

            if (TryReadPackageDependencies(packageJson, out var deps))
            {
                AddIfDependency(deps, "react", frameworks, "React");
                AddIfDependency(deps, "next", frameworks, "Next.js");
                AddIfDependency(deps, "vue", frameworks, "Vue");
                AddIfDependency(deps, "@angular/core", frameworks, "Angular");
                AddIfDependency(deps, "svelte", frameworks, "Svelte");
                AddIfDependency(deps, "nuxt", frameworks, "Nuxt");
                AddIfDependency(deps, "express", frameworks, "Express");
                AddIfDependency(deps, "fastify", frameworks, "Fastify");
                AddIfDependency(deps, "@nestjs/core", frameworks, "NestJS");
            }
        }

        AddIfExists(root, "go.mod", stackIds, "Go", RecordEvidence, languages, "Go", tools, "Go modules");
        AddIfExists(root, "Cargo.toml", stackIds, "Rust", RecordEvidence, languages, "Rust", tools, "Cargo");

        var pythonFiles = new[] { "pyproject.toml", "requirements.txt", "poetry.lock", "Pipfile", "Pipfile.lock" };
        if (pythonFiles.Any(name => File.Exists(Path.Combine(root, name))))
        {
            stackIds.Add("Python");
            languages.Add("Python");
            foreach (var name in pythonFiles)
            {
                if (File.Exists(Path.Combine(root, name)))
                {
                    RecordEvidence(Path.Combine(root, name));
                }
            }

            var pyproject = Path.Combine(root, "pyproject.toml");
            if (File.Exists(pyproject))
            {
                var pyText = ReadAllTextSafe(pyproject);
                if (ContainsIgnoreCase(pyText, "[tool.poetry]"))
                {
                    tools.Add("Poetry");
                }
                AddIfContains(pyText, "django", frameworks, "Django");
                AddIfContains(pyText, "flask", frameworks, "Flask");
                AddIfContains(pyText, "fastapi", frameworks, "FastAPI");
            }

            var requirements = Path.Combine(root, "requirements.txt");
            if (File.Exists(requirements))
            {
                var reqText = ReadAllTextSafe(requirements);
                AddIfContainsWord(reqText, "django", frameworks, "Django");
                AddIfContainsWord(reqText, "flask", frameworks, "Flask");
                AddIfContainsWord(reqText, "fastapi", frameworks, "FastAPI");
            }
        }

        var gemfile = Path.Combine(root, "Gemfile");
        if (File.Exists(gemfile))
        {
            stackIds.Add("Ruby");
            languages.Add("Ruby");
            RecordEvidence(gemfile);
            var text = ReadAllTextSafe(gemfile);
            AddIfContains(text, "rails", frameworks, "Rails");
            AddIfContains(text, "sinatra", frameworks, "Sinatra");
        }

        var mix = Path.Combine(root, "mix.exs");
        if (File.Exists(mix))
        {
            stackIds.Add("Elixir");
            languages.Add("Elixir");
            RecordEvidence(mix);
            var text = ReadAllTextSafe(mix);
            AddIfContains(text, "phoenix", frameworks, "Phoenix");
        }

        var composer = Path.Combine(root, "composer.json");
        if (File.Exists(composer))
        {
            stackIds.Add("PHP");
            languages.Add("PHP");
            RecordEvidence(composer);
            var text = ReadAllTextSafe(composer);
            AddIfContains(text, "laravel", frameworks, "Laravel");
        }

        var pom = Path.Combine(root, "pom.xml");
        if (File.Exists(pom))
        {
            stackIds.Add("Java");
            languages.Add("Java");
            tools.Add("Maven");
            RecordEvidence(pom);
            var text = ReadAllTextSafe(pom);
            AddIfContains(text, "spring-boot", frameworks, "Spring Boot");
        }

        var gradle = Path.Combine(root, "build.gradle");
        if (File.Exists(gradle))
        {
            stackIds.Add("Java");
            languages.Add("Java");
            tools.Add("Gradle");
            RecordEvidence(gradle);
            var text = ReadAllTextSafe(gradle);
            AddIfContains(text, "spring-boot", frameworks, "Spring Boot");
        }

        var gradleKts = Path.Combine(root, "build.gradle.kts");
        if (File.Exists(gradleKts))
        {
            stackIds.Add("Java");
            languages.Add("Java");
            tools.Add("Gradle");
            RecordEvidence(gradleKts);
            var text = ReadAllTextSafe(gradleKts);
            AddIfContains(text, "spring-boot", frameworks, "Spring Boot");
        }

        foreach (var file in Directory.GetFiles(root, "*.csproj"))
        {
            stackIds.Add(".NET");
            languages.Add("C#");
            RecordEvidence(file);
        }

        foreach (var file in Directory.GetFiles(root, "*.sln"))
        {
            RecordEvidence(file);
        }

        AddIfExists(root, "Dockerfile", tools, "Docker", RecordEvidence);
        AddIfExists(root, "docker-compose.yml", tools, "Docker Compose", RecordEvidence);
        AddIfExists(root, "docker-compose.yaml", tools, "Docker Compose", RecordEvidence);
        AddIfExists(root, "Makefile", tools, "Make", RecordEvidence);

        foreach (var file in Directory.GetFiles(root, "*.tf"))
        {
            tools.Add("Terraform");
            RecordEvidence(file);
        }

        return BuildSummary(stackIds, languages, frameworks, tools, runtimes, packageManagers, evidence);
    }

    private static PrdStackSummary BuildSummary(
        HashSet<string> stackIds,
        HashSet<string> languages,
        HashSet<string> frameworks,
        HashSet<string> tools,
        HashSet<string> runtimes,
        HashSet<string> packageManagers,
        List<string> evidence)
    {
        var stackList = stackIds.OrderBy(item => item, StringComparer.Ordinal).ToList();
        var languageList = languages.OrderBy(item => item, StringComparer.Ordinal).ToList();
        var frameworkList = frameworks.OrderBy(item => item, StringComparer.Ordinal).ToList();
        var toolList = tools.OrderBy(item => item, StringComparer.Ordinal).ToList();
        var runtimeList = runtimes.OrderBy(item => item, StringComparer.Ordinal).ToList();
        var packageList = packageManagers.OrderBy(item => item, StringComparer.Ordinal).ToList();
        var evidenceList = evidence.Distinct(StringComparer.Ordinal).OrderBy(item => item, StringComparer.Ordinal).ToList();
        return new PrdStackSummary(stackList, languageList, frameworkList, toolList, runtimeList, packageList, evidenceList, stackList);
    }

    private static void AddIfExists(
        string root,
        string filename,
        HashSet<string> primary,
        string value,
        Action<string> recordEvidence,
        HashSet<string>? secondary = null,
        string? secondaryValue = null,
        HashSet<string>? tertiary = null,
        string? tertiaryValue = null)
    {
        var path = Path.Combine(root, filename);
        if (!File.Exists(path))
        {
            return;
        }

        primary.Add(value);
        if (secondary != null && !string.IsNullOrWhiteSpace(secondaryValue))
        {
            secondary.Add(secondaryValue);
        }

        if (tertiary != null && !string.IsNullOrWhiteSpace(tertiaryValue))
        {
            tertiary.Add(tertiaryValue);
        }

        recordEvidence(path);
    }

    private static bool TryReadPackageDependencies(string path, out HashSet<string> dependencies)
    {
        dependencies = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        try
        {
            var text = File.ReadAllText(path);
            using var doc = JsonDocument.Parse(text);
            foreach (var section in new[] { "dependencies", "devDependencies", "peerDependencies" })
            {
                if (doc.RootElement.TryGetProperty(section, out var depsElement) && depsElement.ValueKind == JsonValueKind.Object)
                {
                    foreach (var prop in depsElement.EnumerateObject())
                    {
                        dependencies.Add(prop.Name);
                    }
                }
            }
            return true;
        }
        catch (Exception)
        {
            return false;
        }
    }

    private static void AddIfDependency(HashSet<string> dependencies, string name, HashSet<string> output, string value)
    {
        if (dependencies.Contains(name))
        {
            output.Add(value);
        }
    }

    private static string ReadAllTextSafe(string path)
    {
        try
        {
            return File.ReadAllText(path);
        }
        catch
        {
            return string.Empty;
        }
    }

    private static bool ContainsIgnoreCase(string? text, string value)
    {
        return text?.IndexOf(value, StringComparison.OrdinalIgnoreCase) >= 0;
    }

    private static void AddIfContains(string? text, string needle, HashSet<string> output, string value)
    {
        if (ContainsIgnoreCase(text, needle))
        {
            output.Add(value);
        }
    }

    private static void AddIfContainsWord(string? text, string needle, HashSet<string> output, string value)
    {
        if (string.IsNullOrWhiteSpace(text))
        {
            return;
        }

        foreach (var line in text.Split('\n'))
        {
            var trimmed = line.Trim();
            if (trimmed.StartsWith("#", StringComparison.Ordinal))
            {
                continue;
            }

            if (ContainsIgnoreCase(trimmed, needle))
            {
                output.Add(value);
                return;
            }
        }
    }
}
