using System.Net.Http.Json;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Gralph.Notifications;

public sealed class WebhookNotifier
{
    private static readonly HttpClient Client = new()
    {
        Timeout = TimeSpan.FromSeconds(30)
    };

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
    };

    public async Task<bool> NotifyCompleteAsync(NotifyCompleteRequest request, CancellationToken cancellationToken)
    {
        if (request is null)
        {
            throw new ArgumentNullException(nameof(request));
        }

        if (string.IsNullOrWhiteSpace(request.WebhookUrl))
        {
            throw new ArgumentException("Webhook URL is required.", nameof(request));
        }

        var payload = BuildCompletePayload(request);
        return await SendAsync(request.WebhookUrl, payload, cancellationToken).ConfigureAwait(false);
    }

    public async Task<bool> NotifyFailedAsync(NotifyFailedRequest request, CancellationToken cancellationToken)
    {
        if (request is null)
        {
            throw new ArgumentNullException(nameof(request));
        }

        if (string.IsNullOrWhiteSpace(request.WebhookUrl))
        {
            throw new ArgumentException("Webhook URL is required.", nameof(request));
        }

        var payload = BuildFailedPayload(request);
        return await SendAsync(request.WebhookUrl, payload, cancellationToken).ConfigureAwait(false);
    }

    private static async Task<bool> SendAsync(string webhookUrl, object payload, CancellationToken cancellationToken)
    {
        using var content = JsonContent.Create(payload, options: JsonOptions);
        using var response = await Client.PostAsync(webhookUrl, content, cancellationToken).ConfigureAwait(false);
        return response.IsSuccessStatusCode;
    }

    private static object BuildCompletePayload(NotifyCompleteRequest request)
    {
        var timestamp = request.Timestamp ?? DateTimeOffset.UtcNow;
        var duration = FormatDuration(request.Duration);
        var webhookType = DetectWebhookType(request.WebhookUrl);

        return webhookType switch
        {
            WebhookType.Discord => BuildDiscordComplete(request, duration, timestamp),
            WebhookType.Slack => BuildSlackComplete(request, duration, timestamp),
            _ => BuildGenericComplete(request, duration, timestamp)
        };
    }

    private static object BuildFailedPayload(NotifyFailedRequest request)
    {
        var timestamp = request.Timestamp ?? DateTimeOffset.UtcNow;
        var duration = FormatDuration(request.Duration);
        var webhookType = DetectWebhookType(request.WebhookUrl);

        return webhookType switch
        {
            WebhookType.Discord => BuildDiscordFailed(request, duration, timestamp),
            WebhookType.Slack => BuildSlackFailed(request, duration, timestamp),
            _ => BuildGenericFailed(request, duration, timestamp)
        };
    }

    private static WebhookType DetectWebhookType(string? url)
    {
        if (string.IsNullOrWhiteSpace(url))
        {
            return WebhookType.Generic;
        }

        if (url.Contains("discord.com/api/webhooks", StringComparison.OrdinalIgnoreCase)
            || url.Contains("discordapp.com/api/webhooks", StringComparison.OrdinalIgnoreCase))
        {
            return WebhookType.Discord;
        }

        if (url.Contains("hooks.slack.com", StringComparison.OrdinalIgnoreCase))
        {
            return WebhookType.Slack;
        }

        return WebhookType.Generic;
    }

    private static object BuildDiscordComplete(NotifyCompleteRequest request, string duration, DateTimeOffset timestamp)
    {
        return new
        {
            embeds = new[]
            {
                new
                {
                    title = "✅ Gralph Complete",
                    description = $"Session **{request.SessionName}** has finished all tasks successfully.",
                    color = 5763719,
                    fields = new[]
                    {
                        new { name = "Project", value = $"`{request.ProjectDir}`", inline = false },
                        new { name = "Iterations", value = request.Iterations.ToString(), inline = true },
                        new { name = "Duration", value = duration, inline = true }
                    },
                    footer = new { text = "Gralph CLI" },
                    timestamp = timestamp.ToString("O")
                }
            }
        };
    }

    private static object BuildSlackComplete(NotifyCompleteRequest request, string duration, DateTimeOffset timestamp)
    {
        return new
        {
            attachments = new[]
            {
                new
                {
                    color = "#57F287",
                    blocks = new object[]
                    {
                        new
                        {
                            type = "header",
                            text = new { type = "plain_text", text = "✅ Gralph Complete", emoji = true }
                        },
                        new
                        {
                            type = "section",
                            text = new { type = "mrkdwn", text = $"Session *{request.SessionName}* has finished all tasks successfully." }
                        },
                        new
                        {
                            type = "section",
                            fields = new object[]
                            {
                                new { type = "mrkdwn", text = $"*Project:*\n`{request.ProjectDir}`" },
                                new { type = "mrkdwn", text = $"*Iterations:*\n{request.Iterations}" },
                                new { type = "mrkdwn", text = $"*Duration:*\n{duration}" }
                            }
                        },
                        new
                        {
                            type = "context",
                            elements = new object[]
                            {
                                new { type = "mrkdwn", text = $"Gralph CLI • {timestamp:O}" }
                            }
                        }
                    }
                }
            }
        };
    }

    private static object BuildGenericComplete(NotifyCompleteRequest request, string duration, DateTimeOffset timestamp)
    {
        return new
        {
            @event = "complete",
            status = "success",
            session = request.SessionName,
            project = request.ProjectDir,
            iterations = request.Iterations.ToString(),
            duration,
            timestamp = timestamp.ToString("O"),
            message = $"Gralph loop '{request.SessionName}' completed successfully after {request.Iterations} iterations ({duration})"
        };
    }

    private static object BuildDiscordFailed(NotifyFailedRequest request, string duration, DateTimeOffset timestamp)
    {
        var description = BuildFailureDescription(request.Reason, request.SessionName, forDiscord: true);
        return new
        {
            embeds = new[]
            {
                new
                {
                    title = "❌ Gralph Failed",
                    description,
                    color = 15548997,
                    fields = new[]
                    {
                        new { name = "Project", value = $"`{request.ProjectDir}`", inline = false },
                        new { name = "Reason", value = request.Reason, inline = true },
                        new { name = "Iterations", value = $"{request.Iterations}/{request.MaxIterations}", inline = true },
                        new { name = "Remaining Tasks", value = request.RemainingTasks.ToString(), inline = true },
                        new { name = "Duration", value = duration, inline = true }
                    },
                    footer = new { text = "Gralph CLI" },
                    timestamp = timestamp.ToString("O")
                }
            }
        };
    }

    private static object BuildSlackFailed(NotifyFailedRequest request, string duration, DateTimeOffset timestamp)
    {
        var description = BuildFailureDescription(request.Reason, request.SessionName, forDiscord: false);
        return new
        {
            attachments = new[]
            {
                new
                {
                    color = "#ED4245",
                    blocks = new object[]
                    {
                        new
                        {
                            type = "header",
                            text = new { type = "plain_text", text = "❌ Gralph Failed", emoji = true }
                        },
                        new
                        {
                            type = "section",
                            text = new { type = "mrkdwn", text = description }
                        },
                        new
                        {
                            type = "section",
                            fields = new object[]
                            {
                                new { type = "mrkdwn", text = $"*Project:*\n`{request.ProjectDir}`" },
                                new { type = "mrkdwn", text = $"*Reason:*\n{request.Reason}" },
                                new { type = "mrkdwn", text = $"*Iterations:*\n{request.Iterations}/{request.MaxIterations}" },
                                new { type = "mrkdwn", text = $"*Remaining Tasks:*\n{request.RemainingTasks}" },
                                new { type = "mrkdwn", text = $"*Duration:*\n{duration}" }
                            }
                        },
                        new
                        {
                            type = "context",
                            elements = new object[]
                            {
                                new { type = "mrkdwn", text = $"Gralph CLI • {timestamp:O}" }
                            }
                        }
                    }
                }
            }
        };
    }

    private static object BuildGenericFailed(NotifyFailedRequest request, string duration, DateTimeOffset timestamp)
    {
        var message = BuildFailureMessage(request.Reason, request.SessionName, request.Iterations, request.MaxIterations, request.RemainingTasks);
        return new
        {
            @event = "failed",
            status = "failure",
            session = request.SessionName,
            project = request.ProjectDir,
            reason = request.Reason,
            iterations = request.Iterations.ToString(),
            max_iterations = request.MaxIterations.ToString(),
            remaining_tasks = request.RemainingTasks.ToString(),
            duration,
            timestamp = timestamp.ToString("O"),
            message
        };
    }

    private static string BuildFailureDescription(string reason, string sessionName, bool forDiscord)
    {
        var bold = forDiscord ? "**" : "*";
        return reason switch
        {
            "max_iterations" => $"Session {bold}{sessionName}{bold} hit maximum iterations limit.",
            "error" => $"Session {bold}{sessionName}{bold} encountered an error.",
            "manual_stop" => $"Session {bold}{sessionName}{bold} was manually stopped.",
            _ => $"Session {bold}{sessionName}{bold} failed: {reason}"
        };
    }

    private static string BuildFailureMessage(string reason, string sessionName, int iterations, int maxIterations, int remainingTasks)
    {
        return reason switch
        {
            "max_iterations" => $"Gralph loop '{sessionName}' failed: hit max iterations ({iterations}/{maxIterations}) with {remainingTasks} tasks remaining",
            "error" => $"Gralph loop '{sessionName}' failed due to an error after {iterations} iterations",
            "manual_stop" => $"Gralph loop '{sessionName}' was manually stopped after {iterations} iterations with {remainingTasks} tasks remaining",
            _ => $"Gralph loop '{sessionName}' failed: {reason} after {iterations} iterations"
        };
    }

    private static string FormatDuration(TimeSpan duration)
    {
        var totalSeconds = (int)Math.Round(duration.TotalSeconds);
        if (totalSeconds < 0)
        {
            return "unknown";
        }

        var hours = totalSeconds / 3600;
        var minutes = (totalSeconds % 3600) / 60;
        var seconds = totalSeconds % 60;

        if (hours > 0)
        {
            return $"{hours}h {minutes}m {seconds}s";
        }

        if (minutes > 0)
        {
            return $"{minutes}m {seconds}s";
        }

        return $"{seconds}s";
    }

    private enum WebhookType
    {
        Discord,
        Slack,
        Generic
    }
}

public sealed class NotifyCompleteRequest
{
    public string WebhookUrl { get; init; } = string.Empty;
    public string SessionName { get; init; } = string.Empty;
    public string ProjectDir { get; init; } = string.Empty;
    public int Iterations { get; init; }
    public TimeSpan Duration { get; init; }
    public DateTimeOffset? Timestamp { get; init; }
}

public sealed class NotifyFailedRequest
{
    public string WebhookUrl { get; init; } = string.Empty;
    public string SessionName { get; init; } = string.Empty;
    public string ProjectDir { get; init; } = string.Empty;
    public string Reason { get; init; } = "unknown";
    public int Iterations { get; init; }
    public int MaxIterations { get; init; }
    public int RemainingTasks { get; init; }
    public TimeSpan Duration { get; init; }
    public DateTimeOffset? Timestamp { get; init; }
}
