# Sharpliner Task Codegen
This tool is designed to quickly and easily create barebones Azure DevOps Task models [for Sharpliner](https://github.com/sharpliner/sharpliner).  Simply give it the documentation of a task you'd like a model for, and it will output (to the best of its ability) a model that can be used in Sharpliner.
## Example Usage
### Input
```
sharpliner_task_codegen.exe --url https://learn.microsoft.com/en-us/azure/devops/pipelines/tasks/reference/npm-v1?view=azure-pipelines
```
### Output
```
// Auto-Generated using 'sharpliner_task_codegen' version 0.1.0 on Mon, 12 May 2025 11:36:29 -0400
// Source Task: Npm v1
// Source Documentation: https://learn.microsoft.com/en-us/azure/devops/pipelines/tasks/reference/npm-v1?view=azure-pipelines

using Sharpliner.AzureDevOps.Tasks;
using YamlDotNet.Serialization;

// --- Enums ---

/// <summary>
/// Defines options for the command parameter.
/// </summary>
public enum Command {
    [YamlMember(Alias = "ci")]
    Ci,

    [YamlMember(Alias = "install")]
    Install,

    [YamlMember(Alias = "publish")]
    Publish,

    [YamlMember(Alias = "custom")]
    Custom,

}

/// <summary>
/// Defines options for the customRegistry parameter.
/// </summary>
public enum CustomRegistry {
    [YamlMember(Alias = "useNpmrc")]
    UseNpmrc,

    [YamlMember(Alias = "useFeed")]
    UseFeed,

}

/// <summary>
/// Defines options for the publishRegistry parameter.
/// </summary>
public enum PublishRegistry {
    [YamlMember(Alias = "useExternalRegistry")]
    UseExternalRegistry,

    [YamlMember(Alias = "useFeed")]
    UseFeed,

}
/// <summary>
/// Generated C# model for the Azure DevOps task: Npm v1.
/// /// Install and publish npm packages, or run an npm command. Supports npmjs.com and authenticated registries like Azure Artifacts.
/// </summary>
public record class NpmTask : AzureDevOpsTask {
    public NpmTask() : base("Npm@1")
    {
    }
    /// <summary>
    /// Command
    /// </summary>
    [YamlIgnore]
    public Command Command {
        get => GetEnum("command", Command.Install);
        init => SetProperty("command", value);
    }

    /// <summary>
    /// json
    /// </summary>
    [YamlIgnore]
    public string? WorkingDir {
        get => GetString("workingDir");
        init => SetProperty("workingDir", value);
    }

    /// <summary>
    /// Command and arguments
    /// </summary>
    [YamlIgnore]
    public string? CustomCommand {
        get => GetString("customCommand");
        init => SetProperty("customCommand", value);
    }

    /// <summary>
    /// Use when command = install || command = ci || command = publish. Verbose logging
    /// </summary>
    [YamlIgnore]
    public bool? Verbose {
        get => GetBool("verbose");
        init => SetProperty("verbose", value);
    }

    /// <summary>
    /// Use when command = publish && publishRegistry = useFeed. Publish pipeline metadata
    /// </summary>
    [YamlIgnore]
    public bool PublishPackageMetadata {
        get => GetBool("publishPackageMetadata", true);
        init => SetProperty("publishPackageMetadata", value);
    }

    /// <summary>
    /// Use when command = install || command = ci || command = custom. Registries to use
    /// </summary>
    [YamlIgnore]
    public CustomRegistry CustomRegistry {
        get => GetEnum("customRegistry", CustomRegistry.UseNpmrc);
        init => SetProperty("customRegistry", value);
    }

    /// <summary>
    /// Use packages from this Azure Artifacts/TFS registry
    /// </summary>
    [YamlIgnore]
    public string? CustomFeed {
        get => GetString("customFeed");
        init => SetProperty("customFeed", value);
    }

    /// <summary>
    /// Use when (command = install || command = ci || command = custom) && customRegistry = useNpmrc. Credentials for registries outside this organization/collection
    /// </summary>
    [YamlIgnore]
    public string? CustomEndpoint {
        get => GetString("customEndpoint");
        init => SetProperty("customEndpoint", value);
    }

    /// <summary>
    /// Use when command = publish. Registry location
    /// </summary>
    [YamlIgnore]
    public PublishRegistry PublishRegistry {
        get => GetEnum("publishRegistry", PublishRegistry.UseExternalRegistry);
        init => SetProperty("publishRegistry", value);
    }

    /// <summary>
    /// Target registry
    /// </summary>
    [YamlIgnore]
    public string? PublishFeed {
        get => GetString("publishFeed");
        init => SetProperty("publishFeed", value);
    }

    /// <summary>
    /// External Registry
    /// </summary>
    [YamlIgnore]
    public string? PublishEndpoint {
        get => GetString("publishEndpoint");
        init => SetProperty("publishEndpoint", value);
    }
}
```

### Notes
All outputs from this tool are output to stdout.  You can pipe this output into a file, or your clipboard.

This solution is not perfect, and may produce unexpected inconsistancies.  It is recommended to use this as a starting point and refine it manually from there.

The tool will attempt to decipher inputs that are option based and generate enums for them automatically.