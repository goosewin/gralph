using System;
using System.Collections.Generic;
using System.Net;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization.Metadata;
using System.Threading;
using System.Threading.Tasks;
using Gralph.Prd;
using Gralph.State;

namespace Gralph.Server;

public sealed class StatusServer
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = false,
        TypeInfoResolver = new DefaultJsonTypeInfoResolver()
    };

    private readonly StatusServerOptions _options;
    private readonly StateStore _state;
    private readonly IProcessInspector _processInspector;
    private readonly CancellationTokenSource _cts = new();
    private HttpListener? _listener;

    public StatusServer(StatusServerOptions options, StateStore state, IProcessInspector? processInspector = null)
    {
        _options = options ?? throw new ArgumentNullException(nameof(options));
        _state = state ?? throw new ArgumentNullException(nameof(state));
        _processInspector = processInspector ?? new ProcessInspector();
    }

    public int Run()
    {
        var prefix = BuildPrefix(_options.Host, _options.Port);
        var listener = new HttpListener();
        listener.Prefixes.Add(prefix);

        try
        {
            listener.Start();
        }
        catch (HttpListenerException ex)
        {
            Console.Error.WriteLine($"Error: Failed to start server on {prefix}: {ex.Message}");
            return 1;
        }

        _listener = listener;
        Console.CancelKeyPress += OnCancelKeyPress;

        try
        {
            while (!_cts.IsCancellationRequested)
            {
                HttpListenerContext context;
                try
                {
                    context = listener.GetContext();
                }
                catch (HttpListenerException) when (_cts.IsCancellationRequested)
                {
                    break;
                }
                catch (ObjectDisposedException)
                {
                    break;
                }

                _ = Task.Run(() => HandleContextAsync(context));
            }
        }
        finally
        {
            Console.CancelKeyPress -= OnCancelKeyPress;
            try
            {
                listener.Close();
            }
            catch (ObjectDisposedException)
            {
            }
        }

        return 0;
    }

    public void Stop()
    {
        if (_cts.IsCancellationRequested)
        {
            return;
        }

        _cts.Cancel();
        try
        {
            _listener?.Stop();
        }
        catch (ObjectDisposedException)
        {
        }
    }

    private void OnCancelKeyPress(object? sender, ConsoleCancelEventArgs args)
    {
        args.Cancel = true;
        Stop();
    }

    private async Task HandleContextAsync(HttpListenerContext context)
    {
        try
        {
            if (!IsAuthorized(context.Request))
            {
                await WriteErrorAsync(context.Response, 401, "Invalid or missing Bearer token").ConfigureAwait(false);
                return;
            }

            var method = context.Request.HttpMethod.ToUpperInvariant();
            var rawPath = context.Request.Url?.AbsolutePath ?? "/";
            var path = NormalizePath(rawPath);

            if (method == "GET" && path == "/")
            {
                var payload = new JsonObject
                {
                    ["status"] = "ok",
                    ["service"] = "gralph-server"
                };
                await WriteJsonAsync(context.Response, 200, payload).ConfigureAwait(false);
                return;
            }

            if (method == "GET" && path == "/status")
            {
                var sessions = BuildSessionList();
                var payload = new JsonObject
                {
                    ["sessions"] = sessions
                };
                await WriteJsonAsync(context.Response, 200, payload).ConfigureAwait(false);
                return;
            }

            if (method == "GET" && path.StartsWith("/status/", StringComparison.Ordinal))
            {
                var name = DecodePathValue(path["/status/".Length..]);
                if (string.IsNullOrWhiteSpace(name))
                {
                    await WriteErrorAsync(context.Response, 404, "Session not found").ConfigureAwait(false);
                    return;
                }

                var session = _state.GetSession(name);
                if (session is null)
                {
                    await WriteErrorAsync(context.Response, 404, $"Session not found: {name}").ConfigureAwait(false);
                    return;
                }

                var enriched = EnrichSession(session);
                await WriteJsonAsync(context.Response, 200, enriched).ConfigureAwait(false);
                return;
            }

            if (method == "POST" && path.StartsWith("/stop/", StringComparison.Ordinal))
            {
                var name = DecodePathValue(path["/stop/".Length..]);
                if (string.IsNullOrWhiteSpace(name))
                {
                    await WriteErrorAsync(context.Response, 404, "Session not found").ConfigureAwait(false);
                    return;
                }

                var session = _state.GetSession(name);
                if (session is null)
                {
                    await WriteErrorAsync(context.Response, 404, $"Session not found: {name}").ConfigureAwait(false);
                    return;
                }

                StopSession(session, name);
                var payload = new JsonObject
                {
                    ["success"] = true,
                    ["message"] = "Session stopped"
                };
                await WriteJsonAsync(context.Response, 200, payload).ConfigureAwait(false);
                return;
            }

            await WriteErrorAsync(context.Response, 404, $"Unknown endpoint: {method} {rawPath}").ConfigureAwait(false);
        }
        catch (Exception ex)
        {
            try
            {
                await WriteErrorAsync(context.Response, 500, $"Server error: {ex.Message}").ConfigureAwait(false);
            }
            catch
            {
            }
        }
    }

    private JsonArray BuildSessionList()
    {
        var sessions = _state.ListSessions();
        var array = new JsonArray();
        foreach (var session in sessions)
        {
            array.Add(EnrichSession(session));
        }

        return array;
    }

    private JsonObject EnrichSession(JsonObject session)
    {
        var clone = (JsonObject)session.DeepClone();
        var status = GetString(clone, "status") ?? "unknown";
        var isAlive = false;

        if (string.Equals(status, "running", StringComparison.OrdinalIgnoreCase)
            && TryGetInt(clone, "pid", out var pid))
        {
            if (_processInspector.IsAlive(pid))
            {
                isAlive = true;
            }
            else
            {
                status = "stale";
            }
        }

        var remaining = ResolveCurrentRemaining(clone);

        clone["current_remaining"] = remaining;
        clone["is_alive"] = isAlive;
        clone["status"] = status;
        return clone;
    }

    private void StopSession(JsonObject session, string name)
    {
        if (TryGetInt(session, "pid", out var pid) && _processInspector.IsAlive(pid))
        {
            try
            {
                using var process = System.Diagnostics.Process.GetProcessById(pid);
                process.Kill(true);
            }
            catch (Exception ex) when (ex is ArgumentException or InvalidOperationException)
            {
            }
            catch (System.ComponentModel.Win32Exception)
            {
            }
        }

        _state.SetSession(name, new Dictionary<string, object?>
        {
            ["status"] = "stopped",
            ["pid"] = string.Empty,
            ["tmux_session"] = string.Empty
        });
    }

    private int ResolveCurrentRemaining(JsonObject session)
    {
        var dir = GetString(session, "dir") ?? string.Empty;
        var taskFile = GetString(session, "task_file") ?? "PRD.md";

        if (!string.IsNullOrWhiteSpace(dir) && System.IO.Directory.Exists(dir))
        {
            var path = System.IO.Path.Combine(dir, taskFile);
            if (System.IO.File.Exists(path))
            {
                return CountRemainingTasks(path);
            }
        }

        if (TryGetInt(session, "last_task_count", out var lastCount))
        {
            return lastCount;
        }

        var lastCountString = GetString(session, "last_task_count");
        if (!string.IsNullOrWhiteSpace(lastCountString) && int.TryParse(lastCountString, out var parsed))
        {
            return parsed;
        }

        return 0;
    }

    private static int CountRemainingTasks(string taskFilePath)
    {
        if (string.IsNullOrWhiteSpace(taskFilePath) || !System.IO.File.Exists(taskFilePath))
        {
            return 0;
        }

        var blocks = PrdParser.GetTaskBlocks(taskFilePath);
        if (blocks.Count > 0)
        {
            var total = 0;
            foreach (var block in blocks)
            {
                total += block.UncheckedCount;
            }
            return total;
        }

        var count = 0;
        foreach (var line in System.IO.File.ReadLines(taskFilePath))
        {
            if (line.AsSpan().TrimStart().StartsWith("- [ ]", StringComparison.Ordinal))
            {
                count++;
            }
        }

        return count;
    }

    private bool IsAuthorized(HttpListenerRequest request)
    {
        if (string.IsNullOrWhiteSpace(_options.Token))
        {
            return true;
        }

        var header = request.Headers["Authorization"] ?? string.Empty;
        if (!header.StartsWith("Bearer ", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        var token = header["Bearer ".Length..].Trim();
        return string.Equals(token, _options.Token, StringComparison.Ordinal);
    }

    private static string BuildPrefix(string host, int port)
    {
        var normalizedHost = NormalizeHost(host);
        return $"http://{normalizedHost}:{port}/";
    }

    private static string NormalizeHost(string host)
    {
        if (string.IsNullOrWhiteSpace(host))
        {
            return "127.0.0.1";
        }

        if (string.Equals(host, "0.0.0.0", StringComparison.Ordinal))
        {
            return "+";
        }

        if (string.Equals(host, "::", StringComparison.Ordinal))
        {
            return "[::]";
        }

        if (host.Contains(':') && !host.StartsWith("[", StringComparison.Ordinal))
        {
            return $"[{host}]";
        }

        return host;
    }

    private static string NormalizePath(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            return "/";
        }

        if (path.Length > 1 && path.EndsWith("/", StringComparison.Ordinal))
        {
            return path.TrimEnd('/');
        }

        return path;
    }

    private static string DecodePathValue(string value)
    {
        return Uri.UnescapeDataString(value ?? string.Empty);
    }

    private static async Task WriteJsonAsync(HttpListenerResponse response, int statusCode, JsonNode payload)
    {
        var json = payload.ToJsonString(JsonOptions);
        var buffer = Encoding.UTF8.GetBytes(json);
        response.StatusCode = statusCode;
        response.ContentType = "application/json";
        response.ContentEncoding = Encoding.UTF8;
        response.ContentLength64 = buffer.Length;
        await response.OutputStream.WriteAsync(buffer, 0, buffer.Length).ConfigureAwait(false);
        response.OutputStream.Close();
    }

    private static Task WriteErrorAsync(HttpListenerResponse response, int statusCode, string message)
    {
        var payload = new JsonObject
        {
            ["error"] = message
        };
        return WriteJsonAsync(response, statusCode, payload);
    }

    private static string? GetString(JsonObject session, string key)
    {
        if (!session.TryGetPropertyValue(key, out var node) || node is null)
        {
            return null;
        }

        if (node is JsonValue value && value.TryGetValue<string>(out var stringValue))
        {
            return stringValue;
        }

        return node.ToString();
    }

    private static bool TryGetInt(JsonObject session, string key, out int result)
    {
        result = 0;
        if (!session.TryGetPropertyValue(key, out var node) || node is null)
        {
            return false;
        }

        if (node is JsonValue value)
        {
            if (value.TryGetValue<int>(out var intValue))
            {
                result = intValue;
                return true;
            }

            if (value.TryGetValue<long>(out var longValue))
            {
                if (longValue is > int.MinValue and < int.MaxValue)
                {
                    result = (int)longValue;
                    return true;
                }
            }

            if (value.TryGetValue<string>(out var stringValue)
                && int.TryParse(stringValue, out var parsed))
            {
                result = parsed;
                return true;
            }
        }

        return false;
    }
}

public sealed class StatusServerOptions
{
    public string Host { get; set; } = "127.0.0.1";
    public int Port { get; set; } = 8080;
    public string Token { get; set; } = string.Empty;
    public bool Open { get; set; }
}
