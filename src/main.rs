use clap::Parser;
use reqwest;
use scraper::{Html, Selector};
use heck::ToPascalCase;
use regex::Regex;
use lazy_static::lazy_static;
//use std::collections::HashMap; // Keep for potential future use? Or remove if truly unused.

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URL of the Azure DevOps task documentation page
    #[arg(short, long)]
    url: String,

    /// Base class name for the generated C# class
    #[arg(short, long, default_value = "AzureDevOpsTask")]
    base_class: String,

    /// Name for the generated C# class (derived from TaskName if not provided)
    #[arg(short, long)]
    class_name: Option<String>,
}

// --- Data Structures ---

// Holds results from line parsing
struct ParsedTaskInfo {
    task_summary: String,
    task_name: String,
    task_version: String,
    parameters: Vec<ProcessedParameter>,
}

// Final processed info for C# generation (same as before)
#[derive(Debug, Clone)]
struct ProcessedParameter {
    yaml_name: String,
    csharp_name: String,
    description: String,
    csharp_type: String, // Final C# type (e.g., "string", "bool?", "NpmCommand")
    enum_options: Option<Vec<String>>,
    is_nullable: bool,
    getter_default_arg: Option<String>, // Formatted default value for Get*(... , default)
    base_csharp_type: String, // Type without '?'
}

// --- Regex Definitions ---
lazy_static! {
    // Rule 3: Task definition line
    static ref TASK_LINE_RE: Regex = Regex::new(
        r"^- task:\s*(?<TaskName>\w+)@(?<TaskVersion>\d+)$"
    ).expect("Invalid Task Line Regex");

    // Rule 4: Input parameter line
    static ref INPUT_LINE_RE: Regex = Regex::new(
        r"^ {3,}(?:#\s*)?(?<InputName>\w+):\s*.*?#\s*(?<Documentation>.*)$"
        //  ^^^^^ Indentation (3+ spaces)
        //       ^^^^^^^^ Optional comment marker # for optional inputs
        //             ^^^^^^^^^^^^ Input Name capture
        //                   ^^ Colon
        //                     ^^^^^^^ Non-greedy skip of example value/whitespace
        //                            ^^^ The # starting the documentation
        //                               ^^^^^^^^^^^^^^^^^^ Capture the documentation string
    ).expect("Invalid Input Line Regex");

    // For parsing the captured Documentation string (same as METADATA_RE before)
    static ref DOC_METADATA_RE: Regex = Regex::new(
       r"^\s*([^.]+?)\s*\.\s*([^.]+?)\s*\.(?:(?:\s*Default:\s*(.+?)\.?$)|(?:(.*?)(?:\.\s*Default:\s*(.+?))?\s*))\.?$"
    ).expect("Invalid Doc Metadata Regex");
    // Group 1: Type/Options ('ci' | 'install'..., string, boolean)
    // Group 2: Required Status (Required, Optional, Required when...)
    // Group 3: Default value if it's the last part
    // Group 4: Description (if default is not the last part)
    // Group 5: Default value (if preceded by description)
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let start_time = std::time::Instant::now(); // Start timing

    println!("// Fetching documentation from: {}", args.url);
    let html_content = fetch_html(&args.url)?;

    println!("// Extracting YAML snippet text...");
    let yaml_text = extract_yaml_snippet(&html_content)?;

    if yaml_text.is_empty() {
         eprintln!("Error: Could not find or extract YAML snippet (selector: 'div.content code.lang-yaml').");
         return Ok(());
    }

    println!("// Parsing YAML snippet line by line...");
    let parsed_info = parse_yaml_lines(&yaml_text)?;

    if parsed_info.parameters.is_empty() {
        eprintln!("Warning: No input parameters parsed from the snippet.");
        // Decide if we should proceed or stop
    }

    println!("// Generating C# code...");
     // Use parsed TaskName for class name if not provided via CLI arg
     let class_name = args.class_name.unwrap_or_else(|| {
         parsed_info.task_name.to_pascal_case() + "Task"
     });


    let csharp_code = generate_csharp(
        &parsed_info.task_summary,
        &parsed_info.task_name,
        &parsed_info.task_version,
        &parsed_info.parameters,
        &class_name,
        &args.base_class
    )?;

    println!("\n// --- Generated C# Code ---");
    println!("{}", csharp_code);

    let duration = start_time.elapsed();
    println!("// Generation finished in {:?}", duration);

    Ok(())
}

// --- HTTP Fetching (same as before) ---
fn fetch_html(url: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()?;
    client.get(url).send()?.text()
}

// --- HTML Snippet Extraction (same as before) ---
fn extract_yaml_snippet(html: &str) -> Result<String, Box<dyn std::error::Error>> {
     let document = Html::parse_document(html);
    // Selector might need adjustment based on actual page structure for lang-yaml blocks
    let selector = Selector::parse("div.content code.lang-yaml, div.content pre code").map_err(|e| e.to_string())?; // Added fallback selector

    if let Some(code_element) = document.select(&selector).next() {
        // Prefer collecting text directly, often more reliable than parsing spans unless structure is guaranteed
        let yaml_content = code_element.text().collect::<String>();
        Ok(yaml_content)
    } else {
        Ok(String::new()) // Return empty if not found
    }
}


// --- Line-by-Line Parsing Logic ---
fn parse_yaml_lines(yaml_text: &str) -> Result<ParsedTaskInfo, Box<dyn std::error::Error>> {
    let lines: Vec<&str> = yaml_text.lines().collect();
    let mut parameters = Vec::new();
    let mut task_summary = String::from("N/A");
    let mut task_name = String::from("UnknownTask");
    let mut task_version = String::from("0");

    let mut line_iter = lines.into_iter().enumerate(); // Use enumerate for index access

    // Rule 1: Ignore first line (index 0)
    line_iter.next();

    // Rule 2: Task Summary (index 1)
    if let Some((_, line)) = line_iter.next() {
        if let Some(summary) = line.trim().strip_prefix('#') {
            task_summary = summary.trim().to_string();
        } else {
             println!("Warning: Line 2 did not seem to contain the task summary comment: '{}'", line);
        }
    } else {
         println!("Warning: Snippet too short, missing task summary line.");
         // Return default info? Or error?
         return Ok(ParsedTaskInfo { task_summary, task_name, task_version, parameters });
    }


    // Rule 3: Task Definition (index 2)
     if let Some((_, line)) = line_iter.next() {
        if let Some(caps) = TASK_LINE_RE.captures(line.trim()) {
            task_name = caps["TaskName"].to_string();
            task_version = caps["TaskVersion"].to_string();
        } else {
             println!("Warning: Line 3 did not match Task definition regex: '{}'", line);
              // Return? Or continue assuming defaults? Let's continue for now.
        }
     } else {
          println!("Warning: Snippet too short, missing task definition line.");
          return Ok(ParsedTaskInfo { task_summary, task_name, task_version, parameters });
     }

    // Rule 4: Input Parameters (remaining lines)
    for (index, line) in line_iter {
        if let Some(caps) = INPUT_LINE_RE.captures(line) {
            let input_name = caps["InputName"].to_string();
            let documentation = caps["Documentation"].trim().to_string();

            if let Some(processed_param) = parse_input_documentation(&input_name, &documentation) {
                parameters.push(processed_param);
            } else {
                println!("Warning: Failed to parse documentation on line {}: '{}'", index + 1, documentation);
            }
        } else if !line.trim().is_empty() && !line.trim().starts_with("inputs:") && !line.trim().starts_with('#') {
             // Optional: Warn about lines that don't match the expected input format but aren't comments/empty/inputs:
             // println!("Warning: Skipping non-empty, non-input line {}: '{}'", index + 1, line);
        }
    }

    Ok(ParsedTaskInfo { task_summary, task_name, task_version, parameters })
}


// --- Documentation String Parsing ---
fn parse_input_documentation(yaml_name: &str, documentation: &str) -> Option<ProcessedParameter> {
     DOC_METADATA_RE.captures(documentation).and_then(|caps| {
        // --- Extract raw parts from regex ---
        let type_options = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
        let required_status = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();
        let description = caps.get(4).map_or("", |m| m.as_str()).trim().to_string();
        // Default value can be in group 3 or 5
        let default_value_str = caps.get(3).or_else(|| caps.get(5)).map(|m| m.as_str().trim().to_string());
         let final_description = if description.is_empty() && default_value_str.is_some() {
            // If group 4 was empty because default was last (group 3 matched)
             // Try to reconstruct description from the original string? Difficult.
             // For now, leave it empty or use a placeholder.
             // A better regex might capture description more reliably even if default is last.
             format!("Details for {}", yaml_name) // Placeholder description
         } else {
             description
         };


        // --- Process extracted parts ---
        let csharp_name = yaml_name.to_pascal_case();
        let mut enum_options = None;
        let mut base_csharp_type = "string".to_string(); // Default assumption

        if type_options.contains('|') && type_options.starts_with('\'') {
            enum_options = Some(type_options.split('|').map(|s| s.trim().replace('\'', "")).collect());
            base_csharp_type = csharp_name.clone(); // Assume enum type name matches PascalCase property name
        } else if type_options == "boolean" {
            base_csharp_type = "bool".to_string();
        } else if type_options == "string" {
            // If we see this as a string, and it has a default value, try to parse the default value as an int.
            // If it parses, set the type to int, otherwise keep it as a string.
            if default_value_str.is_some()
            {
                match default_value_str.as_ref().unwrap().parse::<i32>() {
                    Ok(_) => {
                        base_csharp_type = "int".to_string();
                    },
                    Err(_) => {
                        base_csharp_type = "string".to_string();
                    }
                }
            }
            else {
                base_csharp_type = "string".to_string();
            }
        } // Add other types like 'object', 'secureFile', 'filePath' etc. if needed

        let is_required = required_status == "Required";
        let is_conditionally_required = required_status.starts_with("Required when");
        let is_optional = required_status == "Optional";

        // Apply Nullability Rule (Rule #1)
        let is_nullable = (is_optional || is_conditionally_required || base_csharp_type == "string") && default_value_str.is_none();

        let csharp_type = if is_nullable {
            format!("{}?", base_csharp_type)
        } else {
            base_csharp_type.clone()
        };

        // Format Default Arg for Getter (Rule #2)
        let mut getter_default_arg = None;
        if !is_nullable && default_value_str.is_some() {
            getter_default_arg = Some(format_default_value(
                default_value_str.as_ref().unwrap(),
                &base_csharp_type,
                enum_options.is_some() // is_enum
            ));
        }

         Some(ProcessedParameter {
            yaml_name: yaml_name.to_string(),
            csharp_name,
            description: final_description,
            csharp_type,
            enum_options,
            is_nullable,
            getter_default_arg,
            base_csharp_type,
        })
    })
}

// --- Default Value Formatting (mostly same as before) ---
fn format_default_value(value: &str, base_type: &str, is_enum: bool) -> String {
    // Handle specific known default values that might not parse correctly otherwise
    // These often appear in YAML examples
    if value == "$(BuildConfiguration)" { return "\"$(BuildConfiguration)\"".to_string(); }
    if value == "$(Build.ArtifactStagingDirectory)/*.nupkg" { return "\"$(Build.ArtifactStagingDirectory)/*.nupkg\"".to_string(); }
    if value == "**/*.csproj" { return "\"**/*.csproj\"".to_string(); }
    if value == "$(Build.ArtifactStagingDirectory)" { return "\"$(Build.ArtifactStagingDirectory)\"".to_string(); }

   match base_type {
       "string" => format!("\"{}\"", value.replace('"', "\\\"")),
       "bool" => value.to_lowercase(), // "true" or "false"
       _ if is_enum => {
           let clean_value = value.trim_matches('\'').to_pascal_case();
           format!("{}.{}", base_type, clean_value)
       }
       _ => value.to_string(), // For int, etc.
   }
}


// --- C# Code Generation (Updated Signature) ---
fn generate_csharp(
    task_summary: &str,
    task_name: &str,
    task_version: &str,
    params: &[ProcessedParameter],
    class_name: &str,
    base_class: &str
) -> Result<String, Box<dyn std::error::Error>> {
     let mut enums_code = String::new();
    let mut properties_code = String::new();

    // --- Generate Enums ---
    for p in params {
        if let Some(options) = &p.enum_options {
            enums_code.push_str(&format!("/// <summary>\n/// Defines options for the {} parameter.\n/// </summary>\n", p.yaml_name));
            enums_code.push_str(&format!("public enum {} {{\n", p.base_csharp_type));
            for option in options {
                 let member_name = option.to_pascal_case();
                 let alias = option.replace('\'', "");
                 enums_code.push_str(&format!("    [YamlMember(Alias = \"{}\")]\n", alias));
                 enums_code.push_str(&format!("    {},\n\n", member_name));
            }
            enums_code.push_str("}\n\n");
        }
     }


    // --- Generate Properties ---
    for p in params {
        let mut description_lines = p.description.lines()
            .map(|l| format!("    /// {}", l.trim()))
            .collect::<Vec<_>>()
            .join("\n");
         // Add the original documentation string as well for reference
         //let doc_comment_line = format!("    /// Raw Doc: {}", documentation_escaped(&p.description)); // Need helper to escape XML chars
         //description_lines.push_str(&format!("\n{}", doc_comment_line));


        properties_code.push_str(&format!("    /// <summary>\n{}\n    /// </summary>\n", description_lines));
        properties_code.push_str("    [YamlIgnore]\n");
        properties_code.push_str(&format!("    public {} {} {{\n", p.csharp_type, p.csharp_name));

        // Getter logic remains the same based on ProcessedParameter fields
         properties_code.push_str("        get => ");
        match p.base_csharp_type.as_str() {
            "string" => {
                if let Some(ref default_arg) = p.getter_default_arg {
                    properties_code.push_str(&format!("GetString(\"{}\", {})!", p.yaml_name, default_arg));
                } else {
                    properties_code.push_str(&format!("GetString(\"{}\")", p.yaml_name));
                }
            }
            "bool" => {
                 if let Some(ref default_arg) = p.getter_default_arg {
                    properties_code.push_str(&format!("GetBool(\"{}\", {})", p.yaml_name, default_arg));
                 } else {
                    properties_code.push_str(&format!("GetBool(\"{}\")", p.yaml_name));
                 }
            }
            "int" => {
                if let Some(ref default_arg) = p.getter_default_arg {
                    properties_code.push_str(&format!("GetInt(\"{}\", {})!.Value", p.yaml_name, default_arg));
                } else {
                    properties_code.push_str(&format!("GetInt(\"{}\")!.Value", p.yaml_name));
                }
            }
            _ => { // Assume Enum
                 if let Some(ref default_arg) = p.getter_default_arg {
                    properties_code.push_str(&format!("GetEnum(\"{}\", {})", p.yaml_name, default_arg));
                 } else {
                    properties_code.push_str(&format!("GetNullableEnum<{}>(\"{}\") /* TODO: Verify GetNullableEnum */", p.base_csharp_type, p.yaml_name));
                 }
            }
        }
        properties_code.push_str(";\n");

        // Setter
        properties_code.push_str(&format!("        init => SetProperty(\"{}\", value);\n", p.yaml_name));
        properties_code.push_str("    }\n\n");
    }

    // --- Assemble Final Class ---
     let class_summary = format!(
        "Generated C# model for the Azure DevOps task: {task_name} v{task_version}.\n/// {task_summary}",
        task_name = task_name,
        task_version = task_version,
        task_summary = task_summary // Already trimmed
    );
     let escaped_class_summary = class_summary.lines()
         .map(|l| format!("/// {}", l))
         .collect::<Vec<_>>()
         .join("\n");

    let final_code = format!(
r#"using Sharpliner.AzureDevOps.Tasks;
using YamlDotNet.Serialization;

// Auto-Generated by Rust tool on {generation_date}
// Source Task: {task_name} v{task_version}

// --- Enums ---

{enums_code}
/// <summary>
{escaped_class_summary}
/// </summary>
public record class {class_name} : {base_class} {{
    public {class_name}() : base("{task_name}@{task_version}")
    {{
    }}
{properties_code}}}
"#,
        generation_date = chrono::Local::now().to_rfc2822(), // Using chrono crate if added
        // generation_date = "Current Date/Time", // Simpler alternative if chrono not added
        task_name = task_name,
        task_version = task_version,
        base_class = base_class,
        enums_code = enums_code.trim(),
        escaped_class_summary = escaped_class_summary,
        class_name = class_name,
        properties_code = properties_code.trim_end()
    );

    Ok(final_code)
}

// Helper to escape XML characters in documentation comments
fn documentation_escaped(doc: &str) -> String {
     doc.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        // Add other replacements if needed
}


// Add chrono to Cargo.toml if using the date feature:
// chrono = { version = "0.4", features = ["serde"] } // Or just "0.4"