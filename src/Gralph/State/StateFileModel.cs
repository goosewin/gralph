using System.Text.Json.Serialization;

namespace Gralph.State;

public sealed class StateFileModel
{
    [JsonPropertyName("sessions")]
    public Dictionary<string, SessionState> Sessions { get; set; } = new(StringComparer.Ordinal);
}
