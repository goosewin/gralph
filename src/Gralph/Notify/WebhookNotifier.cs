using System;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization.Metadata;
using System.Threading;
using System.Threading.Tasks;

namespace Gralph.Notify;

public sealed record CompletionNotification(string SessionName, string ProjectDir, int Iterations, TimeSpan Duration);

public sealed record FailureNotification(
    string SessionName,
    string ProjectDir,
    string Reason,
    int Iterations,
    int MaxIterations,
    int RemainingTasks,
    TimeSpan? Duration);

public static class WebhookNotifier
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = null,
        TypeInfoResolver = new DefaultJsonTypeInfoResolver()
    };

    public static Task<bool> NotifyCompleteAsync(
        CompletionNotification notification,
        string webhookUrl,
        CancellationToken cancellationToken = default)
    {
        if (notification is null)
        {
            throw new ArgumentNullException(nameof(notification));
        }

        var payload = BuildCompletePayload(notification, webhookUrl, DateTimeOffset.Now);
        return SendPayloadAsync(webhookUrl, payload, cancellationToken);
    }

    public static Task<bool> NotifyFailedAsync(
        FailureNotification notification,
        string webhookUrl,
        CancellationToken cancellationToken = default)
    {
        if (notification is null)
        {
            throw new ArgumentNullException(nameof(notification));
        }

        var payload = BuildFailedPayload(notification, webhookUrl, DateTimeOffset.Now);
        return SendPayloadAsync(webhookUrl, payload, cancellationToken);
    }

    private static async Task<bool> SendPayloadAsync(string webhookUrl, JsonObject payload, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(webhookUrl) || payload is null)
        {
            return false;
        }

        using var client = new HttpClient { Timeout = TimeSpan.FromSeconds(30) };
        var content = new StringContent(payload.ToJsonString(JsonOptions), Encoding.UTF8, "application/json");
        using var response = await client.PostAsync(webhookUrl, content, cancellationToken);
        return response.IsSuccessStatusCode;
    }

    private static JsonObject BuildCompletePayload(CompletionNotification notification, string webhookUrl, DateTimeOffset timestamp)
    {
        var duration = FormatDuration(notification.Duration);
        var webhookType = DetectWebhookType(webhookUrl);
        var timeValue = FormatTimestamp(timestamp);

        return webhookType switch
        {
            WebhookType.Discord => BuildDiscordComplete(notification, duration, timeValue),
            WebhookType.Slack => BuildSlackComplete(notification, duration, timeValue),
            _ => BuildGenericComplete(notification, duration, timeValue)
        };
    }

    private static JsonObject BuildFailedPayload(FailureNotification notification, string webhookUrl, DateTimeOffset timestamp)
    {
        var duration = FormatDuration(notification.Duration);
        var webhookType = DetectWebhookType(webhookUrl);
        var timeValue = FormatTimestamp(timestamp);

        return webhookType switch
        {
            WebhookType.Discord => BuildDiscordFailed(notification, duration, timeValue),
            WebhookType.Slack => BuildSlackFailed(notification, duration, timeValue),
            _ => BuildGenericFailed(notification, duration, timeValue)
        };
    }

    private static WebhookType DetectWebhookType(string url)
    {
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

    private static JsonObject BuildDiscordComplete(CompletionNotification notification, string duration, string timestamp)
    {
        var fields = new JsonArray
        {
            new JsonObject
            {
                ["name"] = "Project",
                ["value"] = $"`{notification.ProjectDir}`",
                ["inline"] = false
            },
            new JsonObject
            {
                ["name"] = "Iterations",
                ["value"] = notification.Iterations.ToString(),
                ["inline"] = true
            },
            new JsonObject
            {
                ["name"] = "Duration",
                ["value"] = duration,
                ["inline"] = true
            }
        };

        var embed = new JsonObject
        {
            ["title"] = "✅ Gralph Complete",
            ["description"] = $"Session **{notification.SessionName}** has finished all tasks successfully.",
            ["color"] = 5763719,
            ["fields"] = fields,
            ["footer"] = new JsonObject
            {
                ["text"] = "Gralph CLI"
            },
            ["timestamp"] = timestamp
        };

        return new JsonObject
        {
            ["embeds"] = new JsonArray(embed)
        };
    }

    private static JsonObject BuildSlackComplete(CompletionNotification notification, string duration, string timestamp)
    {
        var blocks = new JsonArray
        {
            new JsonObject
            {
                ["type"] = "header",
                ["text"] = new JsonObject
                {
                    ["type"] = "plain_text",
                    ["text"] = "✅ Gralph Complete",
                    ["emoji"] = true
                }
            },
            new JsonObject
            {
                ["type"] = "section",
                ["text"] = new JsonObject
                {
                    ["type"] = "mrkdwn",
                    ["text"] = $"Session *{notification.SessionName}* has finished all tasks successfully."
                }
            },
            new JsonObject
            {
                ["type"] = "section",
                ["fields"] = new JsonArray
                {
                    new JsonObject
                    {
                        ["type"] = "mrkdwn",
                        ["text"] = $"*Project:*\n`{notification.ProjectDir}`"
                    },
                    new JsonObject
                    {
                        ["type"] = "mrkdwn",
                        ["text"] = $"*Iterations:*\n{notification.Iterations}"
                    },
                    new JsonObject
                    {
                        ["type"] = "mrkdwn",
                        ["text"] = $"*Duration:*\n{duration}"
                    }
                }
            },
            new JsonObject
            {
                ["type"] = "context",
                ["elements"] = new JsonArray
                {
                    new JsonObject
                    {
                        ["type"] = "mrkdwn",
                        ["text"] = $"Gralph CLI • {timestamp}"
                    }
                }
            }
        };

        return new JsonObject
        {
            ["attachments"] = new JsonArray
            {
                new JsonObject
                {
                    ["color"] = "#57F287",
                    ["blocks"] = blocks
                }
            }
        };
    }

    private static JsonObject BuildGenericComplete(CompletionNotification notification, string duration, string timestamp)
    {
        return new JsonObject
        {
            ["event"] = "complete",
            ["status"] = "success",
            ["session"] = notification.SessionName,
            ["project"] = notification.ProjectDir,
            ["iterations"] = notification.Iterations.ToString(),
            ["duration"] = duration,
            ["timestamp"] = timestamp,
            ["message"] = $"Gralph loop '{notification.SessionName}' completed successfully after {notification.Iterations} iterations ({duration})"
        };
    }

    private static JsonObject BuildDiscordFailed(FailureNotification notification, string duration, string timestamp)
    {
        var fields = new JsonArray
        {
            new JsonObject
            {
                ["name"] = "Project",
                ["value"] = $"`{notification.ProjectDir}`",
                ["inline"] = false
            },
            new JsonObject
            {
                ["name"] = "Reason",
                ["value"] = notification.Reason,
                ["inline"] = true
            },
            new JsonObject
            {
                ["name"] = "Iterations",
                ["value"] = FormatIterations(notification.Iterations, notification.MaxIterations),
                ["inline"] = true
            },
            new JsonObject
            {
                ["name"] = "Remaining Tasks",
                ["value"] = FormatCount(notification.RemainingTasks),
                ["inline"] = true
            },
            new JsonObject
            {
                ["name"] = "Duration",
                ["value"] = duration,
                ["inline"] = true
            }
        };

        var embed = new JsonObject
        {
            ["title"] = "❌ Gralph Failed",
            ["description"] = BuildFailureDescription(notification.Reason, notification.SessionName, "**"),
            ["color"] = 15548997,
            ["fields"] = fields,
            ["footer"] = new JsonObject
            {
                ["text"] = "Gralph CLI"
            },
            ["timestamp"] = timestamp
        };

        return new JsonObject
        {
            ["embeds"] = new JsonArray(embed)
        };
    }

    private static JsonObject BuildSlackFailed(FailureNotification notification, string duration, string timestamp)
    {
        var blocks = new JsonArray
        {
            new JsonObject
            {
                ["type"] = "header",
                ["text"] = new JsonObject
                {
                    ["type"] = "plain_text",
                    ["text"] = "❌ Gralph Failed",
                    ["emoji"] = true
                }
            },
            new JsonObject
            {
                ["type"] = "section",
                ["text"] = new JsonObject
                {
                    ["type"] = "mrkdwn",
                    ["text"] = BuildFailureDescription(notification.Reason, notification.SessionName, "*")
                }
            },
            new JsonObject
            {
                ["type"] = "section",
                ["fields"] = new JsonArray
                {
                    new JsonObject
                    {
                        ["type"] = "mrkdwn",
                        ["text"] = $"*Project:*\n`{notification.ProjectDir}`"
                    },
                    new JsonObject
                    {
                        ["type"] = "mrkdwn",
                        ["text"] = $"*Reason:*\n{notification.Reason}"
                    },
                    new JsonObject
                    {
                        ["type"] = "mrkdwn",
                        ["text"] = $"*Iterations:*\n{FormatIterations(notification.Iterations, notification.MaxIterations)}"
                    },
                    new JsonObject
                    {
                        ["type"] = "mrkdwn",
                        ["text"] = $"*Remaining Tasks:*\n{FormatCount(notification.RemainingTasks)}"
                    },
                    new JsonObject
                    {
                        ["type"] = "mrkdwn",
                        ["text"] = $"*Duration:*\n{duration}"
                    }
                }
            },
            new JsonObject
            {
                ["type"] = "context",
                ["elements"] = new JsonArray
                {
                    new JsonObject
                    {
                        ["type"] = "mrkdwn",
                        ["text"] = $"Gralph CLI • {timestamp}"
                    }
                }
            }
        };

        return new JsonObject
        {
            ["attachments"] = new JsonArray
            {
                new JsonObject
                {
                    ["color"] = "#ED4245",
                    ["blocks"] = blocks
                }
            }
        };
    }

    private static JsonObject BuildGenericFailed(FailureNotification notification, string duration, string timestamp)
    {
        return new JsonObject
        {
            ["event"] = "failed",
            ["status"] = "failure",
            ["session"] = notification.SessionName,
            ["project"] = notification.ProjectDir,
            ["reason"] = notification.Reason,
            ["iterations"] = FormatCount(notification.Iterations),
            ["max_iterations"] = FormatCount(notification.MaxIterations),
            ["remaining_tasks"] = FormatCount(notification.RemainingTasks),
            ["duration"] = duration,
            ["timestamp"] = timestamp,
            ["message"] = BuildFailureMessage(notification.Reason, notification.SessionName, notification.Iterations, notification.MaxIterations, notification.RemainingTasks)
        };
    }

    private static string BuildFailureDescription(string reason, string sessionName, string emphasisToken)
    {
        return reason switch
        {
            "max_iterations" => $"Session {emphasisToken}{sessionName}{emphasisToken} hit maximum iterations limit.",
            "error" => $"Session {emphasisToken}{sessionName}{emphasisToken} encountered an error.",
            "manual_stop" => $"Session {emphasisToken}{sessionName}{emphasisToken} was manually stopped.",
            _ => $"Session {emphasisToken}{sessionName}{emphasisToken} failed: {reason}"
        };
    }

    private static string BuildFailureMessage(string reason, string sessionName, int iterations, int maxIterations, int remainingTasks)
    {
        return reason switch
        {
            "max_iterations" => $"Gralph loop '{sessionName}' failed: hit max iterations ({FormatIterations(iterations, maxIterations)}) with {FormatCount(remainingTasks)} tasks remaining",
            "error" => $"Gralph loop '{sessionName}' failed due to an error after {FormatCount(iterations)} iterations",
            "manual_stop" => $"Gralph loop '{sessionName}' was manually stopped after {FormatCount(iterations)} iterations with {FormatCount(remainingTasks)} tasks remaining",
            _ => $"Gralph loop '{sessionName}' failed: {reason} after {FormatCount(iterations)} iterations"
        };
    }

    private static string FormatIterations(int iterations, int maxIterations)
    {
        if (iterations < 0 || maxIterations < 0)
        {
            return "unknown";
        }

        if (iterations == 0 && maxIterations == 0)
        {
            return "unknown";
        }

        if (maxIterations <= 0)
        {
            return iterations.ToString();
        }

        return $"{iterations}/{maxIterations}";
    }

    private static string FormatCount(int count)
    {
        return count >= 0 ? count.ToString() : "unknown";
    }

    private static string FormatDuration(TimeSpan? duration)
    {
        if (duration is null)
        {
            return "unknown";
        }

        var totalSeconds = Math.Max(0, (int)duration.Value.TotalSeconds);
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

    private static string FormatTimestamp(DateTimeOffset timestamp)
    {
        return timestamp.ToString("yyyy-MM-ddTHH:mm:ssK");
    }

    private enum WebhookType
    {
        Discord,
        Slack,
        Generic
    }
}
