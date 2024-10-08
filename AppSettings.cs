namespace LocalArchive;

using System.Text.Json.Serialization;

[JsonSerializable(typeof(AppSettings))]
internal partial class AppSettingsContext : JsonSerializerContext
{
}

internal class AppSettings {
   public List<string>? SelectedIgnorePatterns { get; init; }
   public string? LoarDir { get; init; }
}