using System.Text.Json.Serialization;

namespace Gralph.State;

public sealed class SessionState
{
    [JsonPropertyName("name")]
    public string? Name { get; set; }

    [JsonPropertyName("dir")]
    public string? Dir { get; set; }

    [JsonPropertyName("task_file")]
    public string? TaskFile { get; set; }

    [JsonPropertyName("pid")]
    public int? Pid { get; set; }

    [JsonPropertyName("tmux_session")]
    public string? TmuxSession { get; set; }

    [JsonPropertyName("started_at")]
    public long? StartedAt { get; set; }

    [JsonPropertyName("iteration")]
    public int? Iteration { get; set; }

    [JsonPropertyName("max_iterations")]
    public int? MaxIterations { get; set; }

    [JsonPropertyName("status")]
    public string? Status { get; set; }

    [JsonPropertyName("last_task_count")]
    public int? LastTaskCount { get; set; }

    [JsonPropertyName("completion_marker")]
    public string? CompletionMarker { get; set; }

    [JsonPropertyName("log_file")]
    public string? LogFile { get; set; }
}
