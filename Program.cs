namespace LocalArchive;

using Sharprompt;
using System;
using System.Diagnostics;
using System.Text.Json;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net;
using System.Collections.Generic;


internal abstract class Program {
   private static readonly string appVersion = "0.7.0";
   private static readonly string appName = "loar";
   private static readonly string appDescription = "Local Archive Utility";
   private static readonly string appSettingDir = ".LOAR";
   private static readonly string appSettingFile = $"{appName}.json";
   private static readonly string[] LOAR_FILES = [
      $"/{appName}_*",
      appName,
      appSettingFile,
      appSettingDir,
   ];
   private static AppSettings? appSettings;
   private static readonly List<string> DefaultIgnorePatterns = LOAR_FILES.ToList();
   private static readonly Dictionary<string, string[]> LOAR_PATTERNS_TEMPLATE = new() {
      {
         "Git/GitHub", [
            ".git/",
            ".gitignore",
            ".gitattributes",
            ".gitmodules"
         ]
      }, {
         "Flutter", [
            ".dart_tool/",
            ".flutter-plugins",
            ".packages",
            ".pubspec.lock",
            ".DS_Store",
            "ios/Pods/",
            "xcuserdata/",
            "DerivedData/",
            "Carthage/",
            "fastlane/",
            "Podfile.lock",
         ]
      }, {
         "Visual Studio", [
            "build/",
            "node_modules/",
            "bin/",
            "obj/",
            ".AssemblyAttribute.cs",
            ".DotSettings.user",
            ".vs/",
            ".vscode/"
         ]
      }, {
         "Android Studio", [
            "/.gradle/",
            "android/gradle/wrapper/",
            "android/gradlew"
         ]
      }, {
         "XCode", [
            ".DS_Store",
            "ios/Pods/",
            "xcuserdata/",
            "DerivedData/",
            "Carthage/",
            "fastlane/",
            "Podfile.lock",
         ]
      }, {
         "JetBrains IntelliJ", [
            ".idea/", ".iml",
            ".vs/", ".vscode/",
            "/.gradle/", "android/gradle/wrapper/", "android/gradlew",
            ".DS_Store", "ios/Pods/", "xcuserdata/", "DerivedData/", "Carthage/", "fastlane/", "Podfile.lock",
         ]
      }, {
         "JetBrains Rider", [
            ".idea/", ".iml",
            ".vs/", ".vscode/",
            "build/", "node_modules/",
            "bin/", "obj/",
            ".AssemblyAttribute.cs",
            ".DotSettings.user",
            ".vs/", ".vscode/",
            ".DS_Store"
         ]
      },
   };

   private static void ShowUsage() {
      Console.WriteLine();
      Console.WriteLine(appDescription);
      Console.WriteLine($"version {appVersion}");
      Console.WriteLine();
      Console.WriteLine($"Usage: {appName} [options]");
      Console.WriteLine("Options:");
      Console.WriteLine(" --repo-dir, -r    Specify the root directory of repository to archive (default: current directory)");
      Console.WriteLine(" --out-dir, -o     Specify the output directory for the result archive zip file (default: current directory)");
      Console.WriteLine(" --help, -h        Display this help message");
   }

   private static async Task Main(string?[] args) {

      if (args.Contains("--help") || args.Contains("-h")) {
         ShowUsage();
         return;
      }

      Console.WriteLine();
      Console.WriteLine(appDescription);
      Console.WriteLine($"version {appVersion}");
      Console.WriteLine();

      try {
         var exists = File.Exists(appSettingFile);
         if (!exists) {
            var userSelected = Prompt.MultiSelect("Select patterns to archive", LOAR_PATTERNS_TEMPLATE.Keys);
            var SelectedIgnorePatterns = new List<string>(DefaultIgnorePatterns);
            foreach (var pattern in userSelected) {
               if (!LOAR_PATTERNS_TEMPLATE.TryGetValue(pattern, out var patterns)) continue;
               foreach (var p in patterns) {
                  if (!SelectedIgnorePatterns.Contains(p)) {
                     SelectedIgnorePatterns.Add(p);
                  }
               }
            }
            string loarDir;
            do {
               loarDir = Prompt.Input<string>("Enter directory for your local only files (somewhere secure and safe)");
            } while (!Directory.Exists(loarDir));

            appSettings = new AppSettings() { SelectedIgnorePatterns = SelectedIgnorePatterns, LoarDir = loarDir };
            var json = JsonSerializer.Serialize(appSettings, new JsonSerializerOptions
            {
               WriteIndented = true,
               TypeInfoResolver = AppSettingsContext.Default,
            });
            await File.WriteAllTextAsync(appSettingFile, json);
         }
         else {
            var json = await File.ReadAllTextAsync(appSettingFile);
            appSettings = JsonSerializer.Deserialize(json, AppSettingsContext.Default.AppSettings);
         }
         if (appSettings == null) {
            throw new InvalidOperationException("Failed to load app settings.");
         }

         var repoRoot = GetCommandLineArg(args, "--repo-dir", "-r") ?? Directory.GetCurrentDirectory();
         var zipDir = GetCommandLineArg(args, "--out-dir", "-o") ?? appSettings.LoarDir;
         string? password = null; // GetCommandLineArg(args, "--password", "-p");

         var computerName = Dns.GetHostName();
         if (computerName.Contains('.')) {
            computerName = computerName.Substring(0, computerName.IndexOf('.'));
         }

         var timestamp = DateTime.Now.ToString("yyyy-MM-dd-HHmmss");
         var repoName = Path.GetFileName(repoRoot);
         var zipFilePath = Path.Combine(zipDir!, $"{appName}_{timestamp}_{repoName}_{computerName}.zip");

         var gitTrackedFiles = await GetGitTrackedFilesAsync(repoRoot);
         // add loar.json and loar, .LOAR to .gitignore, if not exists
         var gitIgnoreFile = Path.Combine(repoRoot, ".gitignore");
         if (File.Exists(gitIgnoreFile)) {
            var gitIgnoreContent = await File.ReadAllLinesAsync(gitIgnoreFile);
            var gitIgnore = gitIgnoreContent.ToList();
            if (!gitIgnore.Contains(appSettingFile)) {
               gitIgnore.Add(appSettingFile);
            }
            if (!gitIgnore.Contains(appName)) {
               gitIgnore.Add(appName);
            }
            if (!gitIgnore.Contains(appSettingDir)) {
               gitIgnore.Add(appSettingDir);
            }
            await File.WriteAllLinesAsync(gitIgnoreFile, gitIgnore);
         }
         var allFiles = Directory.EnumerateFiles(repoRoot, "*", SearchOption.AllDirectories)
            .Where(file => (File.GetAttributes(file) & FileAttributes.ReparsePoint) != FileAttributes.ReparsePoint);

         var filesToArchive = new List<string>();
         foreach (var filePath in allFiles) {
            var file = Path.GetRelativePath(repoRoot, filePath);
            if (gitTrackedFiles.Contains(file)) continue;
            if (appSettings.SelectedIgnorePatterns!.Any(pattern => file.Contains(pattern))) {
               continue;
            }
            filesToArchive.Add(filePath);
         }

         if (filesToArchive.Count == 0) {
            Console.WriteLine("No files found to archive.");
            return;
         }

         CreateZipArchive(filesToArchive, zipFilePath, repoRoot, password);
         Console.WriteLine($"Archive created at: {zipFilePath}\n\nPLEASE make sure your ZIP file is in SAFE location, in case your local files may contains some secrets.");
      }
      catch (Exception e) {
         await Console.Error.WriteLineAsync($"Error: {e.Message}");
         Console.WriteLine();
         Console.WriteLine("Use --help or -h for usage information.");
         ShowUsage();
         throw;
      }
   }

   #region Private Methods

   private static void CreateZipArchive(IEnumerable<string> files, string zipFilePath, string repoRoot, string? password = null) {
      using var archive = ZipFile.Open(zipFilePath, ZipArchiveMode.Create);
      foreach (var file in files) {
         var relativePath = Path.GetRelativePath(repoRoot, file);
         archive.CreateEntryFromFile(file, relativePath);

         if (!string.IsNullOrEmpty(password)) {
            // Password protection is not supported in native .NET ZIP. Consider using another library or applying encryption.
            // Additional packages or methods are needed for implementation (e.g., DotNetZip, etc.)
            Console.WriteLine("password option, bot implemented yet.");
         }
      }
   }

   private static async Task<HashSet<string>> GetGitTrackedFilesAsync(string repoRoot) {
      var result = await RunGitCommandAsync("git ls-files", repoRoot);
      return result.ToHashSet();
   }

   private static async Task<IEnumerable<string>> RunGitCommandAsync(string command, string workingDirectory) {
      var isWindows = Environment.OSVersion.Platform == PlatformID.Win32NT;
      var shell = isWindows ? "cmd" : "/bin/bash";
      var shellCommand = isWindows ? $"/c {command}" : $"-c \"{command}\"";

      var psi = new ProcessStartInfo(shell, shellCommand) {
         RedirectStandardOutput = true,
         RedirectStandardError = true,
         WorkingDirectory = workingDirectory,
         UseShellExecute = false,
         CreateNoWindow = true // 윈도우에서 콘솔 창 안 띄우기
      };

      using var process = new Process();
      process.StartInfo = psi;

      var output = new List<string>();
      var errors = new List<string>();

      process.OutputDataReceived += (_, e) => {
         if (!string.IsNullOrEmpty(e.Data)) {
            output.Add(e.Data);
         }
      };

      process.ErrorDataReceived += (_, e) => {
         if (!string.IsNullOrEmpty(e.Data)) {
            errors.Add(e.Data);
         }
      };

      process.Start();
      process.BeginOutputReadLine();
      process.BeginErrorReadLine();

      await process.WaitForExitAsync();

      if (process.ExitCode != 0) {
         throw new InvalidOperationException($"Command failed with exit code {process.ExitCode}: {string.Join(Environment.NewLine, errors)}");
      }

      return output;
   }

   private static string? GetCommandLineArg(string?[] args, string longOption, string shortOption) {
      for (var i = 0; i < args.Length; i++) {
         if (args[i] != longOption && args[i] != shortOption) continue;
         if (i + 1 < args.Length)
            return args[i + 1];
      }

      return null;
   }

   #endregion
}