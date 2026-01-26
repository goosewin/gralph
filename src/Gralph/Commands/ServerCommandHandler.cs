using System.Diagnostics;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;
using Gralph.State;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Hosting;

namespace Gralph.Commands;

public sealed class ServerCommandHandler
{
    private static readonly Regex UncheckedTaskRegex = new("^\\s*-\\s*\\[\\s\\]", RegexOptions.Compiled);
    private readonly StateStore _stateStore;

    public ServerCommandHandler(StateStore stateStore)
    {
        _stateStore = stateStore ?? throw new ArgumentNullException(nameof(stateStore));
    }

    public async Task<int> ExecuteAsync(ServerCommandSettings settings, CancellationToken cancellationToken)
    {
        if (settings is null)
        {
            throw new ArgumentNullException(nameof(settings));
        }

        var host = ResolveHost(settings.Host);
        var port = ResolvePort(settings.Port);
        var token = ResolveToken(settings.Token);
        var open = ResolveOpen(settings.Open);

        if (port < 1 || port > 65535)
        {
            Console.Error.WriteLine($"Error: Invalid port number: {port}");
            return 1;
        }

        if (!IsLocalhost(host) && string.IsNullOrWhiteSpace(token) && !open)
        {
            Console.Error.WriteLine($"Error: Token required when binding to non-localhost address ({host})");
            Console.Error.WriteLine("");
            Console.Error.WriteLine("For security, a token is required when exposing the server to the network.");
            Console.Error.WriteLine("Either:");
            Console.Error.WriteLine("  1. Provide a token: --token <your-secret-token>");
            Console.Error.WriteLine("  2. Explicitly disable security: --open (not recommended)");
            Console.Error.WriteLine("  3. Bind to localhost only: --host 127.0.0.1");
            return 1;
        }

        if (!IsLocalhost(host) && open && string.IsNullOrWhiteSpace(token))
        {
            Console.Error.WriteLine("Warning: Server exposed without authentication (--open flag used)");
            Console.Error.WriteLine("Anyone with network access can view and control your sessions!");
            Console.Error.WriteLine("");
        }

        try
        {
            var builder = WebApplication.CreateBuilder(new WebApplicationOptions
            {
                Args = Array.Empty<string>()
            });

            builder.WebHost.UseUrls(BuildUrl(host, port));

            var app = builder.Build();

            app.Use((context, next) =>
            {
                var origin = context.Request.Headers.Origin.ToString();
                var corsOrigin = ResolveCorsOrigin(origin, host, open);
                if (!string.IsNullOrWhiteSpace(corsOrigin))
                {
                    context.Response.Headers["Access-Control-Allow-Origin"] = corsOrigin;
                    if (!string.Equals(corsOrigin, "*", StringComparison.Ordinal))
                    {
                        context.Response.Headers["Vary"] = "Origin";
                    }

                    context.Response.Headers["Access-Control-Allow-Methods"] = "GET, POST, OPTIONS";
                    context.Response.Headers["Access-Control-Allow-Headers"] = "Authorization, Content-Type";
                    context.Response.Headers["Access-Control-Expose-Headers"] = "Content-Length, Content-Type";
                    context.Response.Headers["Access-Control-Max-Age"] = "86400";
                }

                if (HttpMethods.IsOptions(context.Request.Method))
                {
                    context.Response.StatusCode = StatusCodes.Status204NoContent;
                    return Task.CompletedTask;
                }

                if (!IsAuthorized(context.Request, token))
                {
                    return WriteError(context, StatusCodes.Status401Unauthorized, "Invalid or missing Bearer token");
                }

                return next();
            });

            app.MapGet("/", () => Results.Json(new { status = "ok", service = "gralph-server" }));

            app.MapGet("/status", () =>
            {
                var sessions = _stateStore.ListSessions();
                var payload = sessions.Select(BuildSessionView).ToArray();
                return Results.Json(new { sessions = payload });
            });

            app.MapGet("/status/{name}", (string name) =>
            {
                var session = _stateStore.GetSession(name);
                if (session is null)
                {
                    return Results.Json(new { error = "Session not found" }, statusCode: StatusCodes.Status404NotFound);
                }

                return Results.Json(BuildSessionView(session));
            });

            app.MapPost("/stop/{name}", (string name) =>
            {
                if (!TryStopSession(name, out var error))
                {
                    if (string.Equals(error, "Session not found", StringComparison.Ordinal))
                    {
                        return Results.Json(new { error }, statusCode: StatusCodes.Status404NotFound);
                    }

                    return Results.Json(new { error = error ?? "Failed to stop session" }, statusCode: StatusCodes.Status500InternalServerError);
                }

                return Results.Json(new { success = true, message = "Session stopped" });
            });

            Console.WriteLine($"Starting gralph status server on {host}:{port}...");
            Console.WriteLine("Endpoints:");
            Console.WriteLine("  GET  /status        - Get all sessions");
            Console.WriteLine("  GET  /status/:name  - Get specific session");
            Console.WriteLine("  POST /stop/:name    - Stop a session");
            Console.WriteLine(string.IsNullOrWhiteSpace(token)
                ? "Authentication: None (use --token to enable)"
                : "Authentication: Bearer token required");
            Console.WriteLine("");
            Console.WriteLine("Press Ctrl+C to stop");
            Console.WriteLine("");

            await app.RunAsync(cancellationToken);
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: Failed to start server: {ex.Message}");
            return 1;
        }
    }

    private SessionStatusView BuildSessionView(SessionState session)
    {
        var status = session.Status ?? "unknown";
        var isAlive = false;
        if (string.Equals(status, "running", StringComparison.OrdinalIgnoreCase) && session.Pid is > 0)
        {
            isAlive = IsProcessAlive(session.Pid.Value);
            if (!isAlive)
            {
                status = "stale";
            }
        }

        var taskFile = string.IsNullOrWhiteSpace(session.TaskFile) ? "PRD.md" : session.TaskFile;

        return new SessionStatusView
        {
            Name = session.Name,
            Dir = session.Dir,
            TaskFile = taskFile,
            Pid = session.Pid,
            TmuxSession = session.TmuxSession,
            StartedAt = session.StartedAt,
            Iteration = session.Iteration,
            MaxIterations = session.MaxIterations,
            Status = status,
            LastTaskCount = session.LastTaskCount,
            CompletionMarker = session.CompletionMarker,
            LogFile = session.LogFile,
            Backend = session.Backend,
            Model = session.Model,
            Variant = session.Variant,
            Webhook = session.Webhook,
            CurrentRemaining = GetRemainingTasks(session, taskFile),
            IsAlive = isAlive
        };
    }

    private bool TryStopSession(string name, out string? error)
    {
        error = null;
        if (string.IsNullOrWhiteSpace(name))
        {
            error = "Session not found";
            return false;
        }

        var session = _stateStore.GetSession(name);
        if (session is null)
        {
            error = "Session not found";
            return false;
        }

        if (session.Pid is int pid and > 0)
        {
            TryKillProcess(pid);
        }

        _stateStore.SetSession(name, existing =>
        {
            existing.Status = "stopped";
            existing.Pid = null;
            existing.TmuxSession = null;
        });

        return true;
    }

    private static bool TryKillProcess(int pid)
    {
        try
        {
            var process = Process.GetProcessById(pid);
            if (process.HasExited)
            {
                return false;
            }

            process.Kill(true);
            process.WaitForExit(TimeSpan.FromSeconds(5));
            return true;
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (InvalidOperationException)
        {
            return false;
        }
        catch (Exception)
        {
            return false;
        }
    }

    private static bool IsAuthorized(HttpRequest request, string? token)
    {
        if (string.IsNullOrWhiteSpace(token))
        {
            return true;
        }

        var authHeader = request.Headers.Authorization.ToString();
        if (string.IsNullOrWhiteSpace(authHeader))
        {
            return false;
        }

        if (!authHeader.StartsWith("Bearer ", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        var provided = authHeader["Bearer ".Length..].Trim();
        return string.Equals(provided, token, StringComparison.Ordinal);
    }

    private static Task WriteError(HttpContext context, int statusCode, string message)
    {
        context.Response.StatusCode = statusCode;
        context.Response.ContentType = "application/json";
        return context.Response.WriteAsJsonAsync(new { error = message });
    }

    private static string? ResolveCorsOrigin(string? origin, string host, bool open)
    {
        if (string.IsNullOrWhiteSpace(origin))
        {
            return null;
        }

        if (open)
        {
            return "*";
        }

        if (string.Equals(origin, "http://localhost", StringComparison.OrdinalIgnoreCase)
            || string.Equals(origin, "http://127.0.0.1", StringComparison.OrdinalIgnoreCase)
            || string.Equals(origin, "http://[::1]", StringComparison.OrdinalIgnoreCase))
        {
            return origin;
        }

        if (!string.IsNullOrWhiteSpace(host)
            && !string.Equals(host, "0.0.0.0", StringComparison.OrdinalIgnoreCase)
            && !string.Equals(host, "::", StringComparison.OrdinalIgnoreCase))
        {
            var expected = $"http://{host}";
            if (string.Equals(origin, expected, StringComparison.OrdinalIgnoreCase))
            {
                return origin;
            }
        }

        return null;
    }

    private static int GetRemainingTasks(SessionState session, string taskFile)
    {
        if (string.IsNullOrWhiteSpace(session.Dir))
        {
            return 0;
        }

        var path = Path.IsPathRooted(taskFile)
            ? taskFile
            : Path.Combine(session.Dir, taskFile);

        if (!File.Exists(path))
        {
            return 0;
        }

        var count = 0;
        foreach (var line in File.ReadLines(path))
        {
            if (UncheckedTaskRegex.IsMatch(line))
            {
                count++;
            }
        }

        return count;
    }

    private static bool IsLocalhost(string host)
    {
        return string.Equals(host, "127.0.0.1", StringComparison.OrdinalIgnoreCase)
            || string.Equals(host, "localhost", StringComparison.OrdinalIgnoreCase)
            || string.Equals(host, "::1", StringComparison.OrdinalIgnoreCase);
    }

    private static string ResolveHost(string? provided)
    {
        if (!string.IsNullOrWhiteSpace(provided))
        {
            return provided.Trim();
        }

        var env = Environment.GetEnvironmentVariable("GRALPH_SERVER_HOST");
        if (!string.IsNullOrWhiteSpace(env))
        {
            return env.Trim();
        }

        return "127.0.0.1";
    }

    private static int ResolvePort(int? provided)
    {
        if (provided.HasValue)
        {
            return provided.Value;
        }

        var env = Environment.GetEnvironmentVariable("GRALPH_SERVER_PORT");
        if (!string.IsNullOrWhiteSpace(env) && int.TryParse(env, out var parsed))
        {
            return parsed;
        }

        return 8080;
    }

    private static string? ResolveToken(string? provided)
    {
        if (!string.IsNullOrWhiteSpace(provided))
        {
            return provided.Trim();
        }

        var env = Environment.GetEnvironmentVariable("GRALPH_SERVER_TOKEN");
        return string.IsNullOrWhiteSpace(env) ? null : env.Trim();
    }

    private static bool ResolveOpen(bool open)
    {
        if (open)
        {
            return true;
        }

        var env = Environment.GetEnvironmentVariable("GRALPH_SERVER_OPEN");
        return string.Equals(env, "true", StringComparison.OrdinalIgnoreCase);
    }

    private static string BuildUrl(string host, int port)
    {
        var formattedHost = host;
        if (host.Contains(':') && !host.StartsWith('['))
        {
            formattedHost = $"[{host}]";
        }

        return $"http://{formattedHost}:{port}";
    }

    private static bool IsProcessAlive(int pid)
    {
        try
        {
            var process = Process.GetProcessById(pid);
            return !process.HasExited;
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (InvalidOperationException)
        {
            return false;
        }
    }
}

public sealed class ServerCommandSettings
{
    public string? Host { get; init; }
    public int? Port { get; init; }
    public string? Token { get; init; }
    public bool Open { get; init; }
}

public sealed class SessionStatusView
{
    [JsonPropertyName("name")]
    public string? Name { get; init; }

    [JsonPropertyName("dir")]
    public string? Dir { get; init; }

    [JsonPropertyName("task_file")]
    public string? TaskFile { get; init; }

    [JsonPropertyName("pid")]
    public int? Pid { get; init; }

    [JsonPropertyName("tmux_session")]
    public string? TmuxSession { get; init; }

    [JsonPropertyName("started_at")]
    public long? StartedAt { get; init; }

    [JsonPropertyName("iteration")]
    public int? Iteration { get; init; }

    [JsonPropertyName("max_iterations")]
    public int? MaxIterations { get; init; }

    [JsonPropertyName("status")]
    public string? Status { get; init; }

    [JsonPropertyName("last_task_count")]
    public int? LastTaskCount { get; init; }

    [JsonPropertyName("completion_marker")]
    public string? CompletionMarker { get; init; }

    [JsonPropertyName("log_file")]
    public string? LogFile { get; init; }

    [JsonPropertyName("backend")]
    public string? Backend { get; init; }

    [JsonPropertyName("model")]
    public string? Model { get; init; }

    [JsonPropertyName("variant")]
    public string? Variant { get; init; }

    [JsonPropertyName("webhook")]
    public string? Webhook { get; init; }

    [JsonPropertyName("current_remaining")]
    public int CurrentRemaining { get; init; }

    [JsonPropertyName("is_alive")]
    public bool IsAlive { get; init; }
}
