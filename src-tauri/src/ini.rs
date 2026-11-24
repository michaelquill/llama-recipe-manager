//! `.ini` presets: recipes serialised as llama-server `--models-preset` configuration.
//!
//! Keys within a preset section correspond to llama-server arguments without leading dashes.
//! Boolean flags default to `true`.

use std::collections::HashMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::db::Recipe;

pub const EXTENSION: &str = "ini";

const FORBIDDEN_INI_FLAGS: &[&str] = &[
    "port",
    "p",
    "host",
    "m",
    "model",
    "model-path",
    "mmproj",
    "mm",
    "mmproj-url",
    "mmu",
    "log-file",
    "api-key",
    "api-key-file",
    "ssl-cert-file",
    "ssl-key-file",
    "hf-token",
    "hft",
    "api-prefix",
    "timeout",
    "to",
    "log-verbosity",
    "lv",
    "webui",
    "no-webui",
    "metrics",
    "slots",
    "no-slots",
];

const SHORT_FLAGS: &[&str] = &["c", "t", "b", "n", "ngl", "np"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedIniPreset {
    pub name: String,
    pub description: String,
    pub model_path: String,
    pub mmproj_path: String,
    pub command: String,
    pub gpu_info: String,
    pub tags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedIniFile {
    pub presets: Vec<ParsedIniPreset>,
    pub root_variables: HashMap<String, String>,
}

/// Appends `.ini` if missing.
#[must_use]
pub fn with_ini_extension(path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.extension().is_some_and(|e| e == EXTENSION) {
        path.to_path_buf()
    } else {
        let mut name = path.as_os_str().to_os_string();
        name.push(".");
        name.push(EXTENSION);
        PathBuf::from(name)
    }
}

/// Replaces all `{var_name}` occurrences in `val` with values from `sym_table`.
#[must_use]
pub fn expand_vars(val: &str, sym_table: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(val.len());
    let mut chars = val.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '{' {
            if let Some(end_rel) = val[i + 1..].find('}') {
                let end = i + 1 + end_rel;
                let var_name = &val[i + 1..end];
                if let Some(replacement) = sym_table.get(var_name) {
                    result.push_str(replacement);
                } else {
                    result.push_str(&val[i..=end]);
                }
                while let Some(&(next_i, _)) = chars.peek() {
                    if next_i <= end {
                        chars.next();
                    } else {
                        break;
                    }
                }
                continue;
            }
        }
        result.push(c);
    }
    result
}

#[derive(Debug)]
struct RawSection {
    name: String,
    comments: String,
    pairs: Vec<(String, String)>,
}

#[must_use]
fn is_variable_key(key: &str) -> bool {
    key == "llama-root"
        || key == "llama-cpp-root"
        || key.ends_with("-root")
        || key.ends_with("-dir")
        || key.ends_with("-path")
}

#[must_use]
fn extract_placeholder_names(val: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut i = 0;
    while let Some(start) = val[i..].find('{') {
        let abs_start = i + start;
        if let Some(end) = val[abs_start + 1..].find('}') {
            let abs_end = abs_start + 1 + end;
            let name = &val[abs_start + 1..abs_end];
            if !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                names.push(name.to_string());
            }
            i = abs_end + 1;
        } else {
            break;
        }
    }
    names
}

#[must_use]
fn maybe_quote(val: &str) -> String {
    if val.contains(' ') || val.contains('"') || val.contains('\'') {
        shell_words::quote(val).to_string()
    } else {
        val.to_string()
    }
}

/// Parses an INI string containing llama-server model presets.
///
/// Expands `{var}` placeholders using discovered variables, section overrides,
/// and optional user-supplied `remaps`.
pub fn parse_ini_presets(content: &str, remaps: &HashMap<String, String>) -> ParsedIniFile {
    let mut sections: Vec<RawSection> = Vec::new();
    let mut current_section: Option<RawSection> = None;
    let mut accumulated_comments: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with(';') || trimmed.starts_with('#') {
            let comment_text = trimmed.trim_start_matches([';', '#', ' ']);
            accumulated_comments.push(comment_text.to_string());
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let sec_name = trimmed[1..trimmed.len() - 1].trim();
            if let Some(prev) = current_section.take() {
                sections.push(prev);
            }
            current_section = Some(RawSection {
                name: sec_name.to_string(),
                comments: accumulated_comments.join("\n"),
                pairs: Vec::new(),
            });
            accumulated_comments.clear();
            continue;
        }

        if let Some((k, v)) = trimmed.split_once('=') {
            if let Some(ref mut sec) = current_section {
                sec.pairs.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
    }

    if let Some(prev) = current_section {
        sections.push(prev);
    }

    // Discover root variables from [*] and other sections
    let mut global_pairs: Vec<(String, String)> = Vec::new();
    let mut root_vars: HashMap<String, String> = HashMap::new();

    for sec in &sections {
        if sec.name == "*" {
            for (k, v) in &sec.pairs {
                global_pairs.push((k.clone(), v.clone()));
                if is_variable_key(k) {
                    root_vars.insert(k.clone(), v.clone());
                }
            }
        } else {
            for (k, v) in &sec.pairs {
                if is_variable_key(k) {
                    root_vars.insert(k.clone(), v.clone());
                }
            }
        }
    }

    // Scan for any additional {var} references
    for sec in &sections {
        for (_, v) in &sec.pairs {
            for var_name in extract_placeholder_names(v) {
                root_vars.entry(var_name).or_default();
            }
        }
    }

    // Build preset list
    let mut presets = Vec::new();

    for sec in &sections {
        if sec.name == "*" {
            continue;
        }

        // Merge global defaults with section pairs
        let mut sec_dict: HashMap<String, String> = HashMap::new();
        let mut key_order: Vec<String> = Vec::new();

        for (k, v) in &global_pairs {
            if !sec_dict.contains_key(k) {
                key_order.push(k.clone());
            }
            sec_dict.insert(k.clone(), v.clone());
        }

        for (k, v) in &sec.pairs {
            if !sec_dict.contains_key(k) {
                key_order.push(k.clone());
            }
            sec_dict.insert(k.clone(), v.clone());
        }

        // Build symbol table for this section
        let mut sym_table = root_vars.clone();
        for (k, v) in &sec_dict {
            if is_variable_key(k) || root_vars.contains_key(k) {
                sym_table.insert(k.clone(), v.clone());
            }
        }
        for (k, v) in remaps {
            if !v.is_empty() {
                sym_table.insert(k.clone(), v.clone());
            }
        }

        // Extract model path
        let Some(raw_model) = sec_dict.get("model") else {
            continue;
        };
        let model_path = expand_vars(raw_model, &sym_table);
        if model_path.is_empty() {
            continue;
        }

        let mmproj_path = sec_dict
            .get("mmproj")
            .map(|raw| expand_vars(raw, &sym_table))
            .unwrap_or_default();

        let mut cmd_parts: Vec<String> = Vec::new();
        let mut tags_list: Vec<String> = Vec::new();

        if let Some(runner) = sec_dict.get("runner") {
            tags_list.push(format!("runner:{runner}"));
        }

        for k in &key_order {
            if k == "model" || k == "mmproj" || k == "alias" || k == "runner" {
                continue;
            }
            if is_variable_key(k) || root_vars.contains_key(k) {
                continue;
            }
            if FORBIDDEN_INI_FLAGS.contains(&k.as_str()) {
                continue;
            }

            let Some(raw_v) = sec_dict.get(k) else {
                continue;
            };
            let v = expand_vars(raw_v, &sym_table);

            if k == "spec-draft-model" {
                cmd_parts.push("--model-draft".to_string());
                cmd_parts.push(maybe_quote(&v));
                continue;
            }

            if v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on") {
                if k == "flash-attn" || k == "fit" || k == "cnv" || k == "reasoning-preserve" {
                    cmd_parts.push(format!("--{k} {}", v.to_ascii_lowercase()));
                } else {
                    cmd_parts.push(format!("--{k}"));
                }
            } else if v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("off") {
                if k != "mlock" {
                    cmd_parts.push(format!("--no-{k}"));
                }
            } else if SHORT_FLAGS.contains(&k.as_str()) {
                cmd_parts.push(format!("-{k} {}", maybe_quote(&v)));
            } else {
                cmd_parts.push(format!("--{k} {}", maybe_quote(&v)));
            }
        }

        let command = cmd_parts.join(" ");

        presets.push(ParsedIniPreset {
            name: sec.name.clone(),
            description: sec.comments.clone(),
            model_path,
            mmproj_path,
            command,
            gpu_info: String::new(),
            tags: tags_list.join(", "),
        });
    }

    ParsedIniFile {
        presets,
        root_variables: root_vars,
    }
}

/// Converts a single recipe into a llama-server preset INI section.
pub fn to_ini(recipe: &Recipe) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("; Auto-generated llama.cpp model configuration preset\n");
    out.push_str("; Generated by Llama Recipe Manager\n");
    out.push_str("; For usage with: llama-server --models-preset <file.ini>\n\n");
    format_recipe_section(&mut out, recipe)?;
    Ok(out)
}

/// Converts multiple recipes into a unified llama-server models preset INI file with [*] defaults.
pub fn all_to_ini(recipes: &[Recipe]) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("; Auto-generated llama.cpp models configuration file\n");
    out.push_str("; Exported from Llama Recipe Manager\n");
    out.push_str("; For usage with: llama-server --models-preset <file.ini>\n\n");
    out.push_str("# Shared global defaults for all models\n");
    out.push_str("[*]\n");
    out.push_str("n-gpu-layers = -1\n");
    out.push_str("jinja = true\n");
    out.push_str("flash-attn = on\n\n");

    for (i, recipe) in recipes.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        format_recipe_section(&mut out, recipe)?;
    }

    Ok(out)
}

fn format_recipe_section(out: &mut String, recipe: &Recipe) -> Result<(), String> {
    let _ = writeln!(out, "[{}]", recipe.name);
    let _ = writeln!(out, "model = {}", recipe.model_path);

    if !recipe.mmproj_path.is_empty() {
        let _ = writeln!(out, "mmproj = {}", recipe.mmproj_path);
    }

    if !recipe.description.is_empty() {
        let _ = writeln!(
            out,
            "; description: {}",
            recipe.description.replace('\n', " ")
        );
    }
    if !recipe.tags.is_empty() {
        let _ = writeln!(out, "; tags: {}", recipe.tags);
    }
    if !recipe.gpu_info.is_empty() {
        let _ = writeln!(out, "; gpu: {}", recipe.gpu_info);
    }

    let tokens = shell_words::split(&recipe.command)
        .map_err(|e| format!("Failed to parse recipe command: {e}"))?;

    let args: &[String] = if !tokens.is_empty()
        && (tokens[0] == "llama-server" || tokens[0].ends_with("/llama-server"))
    {
        &tokens[1..]
    } else {
        &tokens
    };

    let mut alias_set = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        if let Some(rest) = arg.strip_prefix("--").or_else(|| arg.strip_prefix('-')) {
            if let Some((key, val)) = rest.split_once('=') {
                if key != "m" && key != "model" && key != "mmproj" {
                    if key == "a" || key == "alias" {
                        alias_set = true;
                        let _ = writeln!(out, "alias = {val}");
                    } else {
                        let _ = writeln!(out, "{key} = {val}");
                    }
                }
                i += 1;
            } else {
                let key = rest;
                if key == "m" || key == "model" || key == "mmproj" {
                    if i + 1 < args.len() && is_value_token(&args[i + 1]) {
                        i += 2;
                    } else {
                        i += 1;
                    }
                } else if i + 1 < args.len() && is_value_token(&args[i + 1]) {
                    let val = &args[i + 1];
                    if key == "a" || key == "alias" {
                        alias_set = true;
                        let _ = writeln!(out, "alias = {val}");
                    } else {
                        let _ = writeln!(out, "{key} = {val}");
                    }
                    i += 2;
                } else {
                    let _ = writeln!(out, "{key} = true");
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }

    if !alias_set {
        let _ = writeln!(out, "alias = {}", recipe.name);
    }

    Ok(())
}

fn is_value_token(token: &str) -> bool {
    if !token.starts_with('-') {
        return true;
    }
    // Negative numbers like -1 or -0.5 are values, not flag options
    let after_minus = &token[1..];
    after_minus
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_recipe() -> Recipe {
        Recipe {
            id: "test-id".to_string(),
            name: "Gemma 4 12B".to_string(),
            description: "Fast reasoning model".to_string(),
            command: "--ctx-size 16384 -ngl 99 --temp 0.7 --top-k 50 --jinja --chat-template-kwargs '{\"enable_thinking\":true}'".to_string(),
            model_path: "/models/gemma-4-12b.gguf".to_string(),
            mmproj_path: "/models/mmproj.gguf".to_string(),
            gpu_info: "RTX 4090".to_string(),
            tags: "gemma, 12b".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            pinned: false,
            last_used_at: String::new(),
        }
    }

    #[test]
    fn formats_single_recipe_ini() {
        let text = to_ini(&sample_recipe()).unwrap();
        assert!(text.contains("[Gemma 4 12B]"));
        assert!(text.contains("model = /models/gemma-4-12b.gguf"));
        assert!(text.contains("mmproj = /models/mmproj.gguf"));
        assert!(text.contains("ctx-size = 16384"));
        assert!(text.contains("ngl = 99"));
        assert!(text.contains("temp = 0.7"));
        assert!(text.contains("top-k = 50"));
        assert!(text.contains("jinja = true"));
        assert!(text.contains("chat-template-kwargs = {\"enable_thinking\":true}"));
        assert!(text.contains("alias = Gemma 4 12B"));
        assert!(text.contains("; description: Fast reasoning model"));
    }

    #[test]
    fn formats_all_recipes_ini_with_global_defaults() {
        let recipes = vec![sample_recipe()];
        let text = all_to_ini(&recipes).unwrap();
        assert!(text.contains("[*]"));
        assert!(text.contains("n-gpu-layers = -1"));
        assert!(text.contains("flash-attn = on"));
        assert!(text.contains("[Gemma 4 12B]"));
    }

    #[test]
    fn handles_explicit_alias_and_negative_numbers() {
        let mut r = sample_recipe();
        r.command = "-a custom-alias -ngl -1".to_string();
        let text = to_ini(&r).unwrap();
        assert!(text.contains("alias = custom-alias"));
        assert!(text.contains("ngl = -1"));
    }

    #[test]
    fn parses_ini_with_root_expansion_and_global_defaults() {
        let ini = r"
[*]
llama-root = /opt/models
ngl = 999
flash-attn = on
port = 8080

; Top-level description
[gemma4]
model = {llama-root}/gemma-4-12b.gguf
mmproj = {llama-root}/mmproj.gguf
c = 65536
";
        let remaps = HashMap::new();
        let parsed = parse_ini_presets(ini, &remaps);
        assert_eq!(parsed.presets.len(), 1);
        let p = &parsed.presets[0];
        assert_eq!(p.name, "gemma4");
        assert_eq!(p.model_path, "/opt/models/gemma-4-12b.gguf");
        assert_eq!(p.mmproj_path, "/opt/models/mmproj.gguf");
        assert_eq!(p.description, "Top-level description");
        assert!(p.command.contains("-ngl 999"));
        assert!(p.command.contains("-c 65536"));
        assert!(p.command.contains("--flash-attn on"));
        assert!(!p.command.contains("8080")); // forbidden port filtered out
    }

    #[test]
    fn handles_variable_remapping() {
        let ini = r"
[*]
llama-root = /home/user/models

[model1]
model = {llama-root}/test.gguf
";
        let mut remaps = HashMap::new();
        remaps.insert("llama-root".to_string(), "/media/ssd/models".to_string());
        let parsed = parse_ini_presets(ini, &remaps);
        assert_eq!(parsed.presets[0].model_path, "/media/ssd/models/test.gguf");
    }

    #[test]
    fn handles_spec_draft_model_and_comments() {
        let ini = r"
[*]
llama-root = /models

; High performance reasoning
; Fits in 16GB VRAM
[reasoning-model]
model = {llama-root}/base.gguf
spec-draft-model = {llama-root}/draft.gguf
spec-type = draft-mtp
temp = 1.0
runner = diffusion
";
        let remaps = HashMap::new();
        let parsed = parse_ini_presets(ini, &remaps);
        let p = &parsed.presets[0];
        assert_eq!(p.name, "reasoning-model");
        assert!(p.description.contains("High performance reasoning"));
        assert!(p.description.contains("Fits in 16GB VRAM"));
        assert!(p.command.contains("--model-draft /models/draft.gguf"));
        assert!(p.command.contains("--spec-type draft-mtp"));
        assert!(p.command.contains("--temp 1.0"));
        assert!(p.tags.contains("runner:diffusion"));
    }

    #[test]
    fn parses_real_sample_file() {
        let sample_path = Path::new("../example/example_model-configs.ini");
        if sample_path.exists() {
            let content = std::fs::read_to_string(sample_path).unwrap();
            let remaps = HashMap::new();
            let parsed = parse_ini_presets(&content, &remaps);
            assert_eq!(parsed.presets.len(), 277);
            assert!(parsed.root_variables.contains_key("llama-root"));
            assert_eq!(
                parsed.root_variables.get("llama-root").unwrap(),
                "/home/user/llama.cpp/models"
            );
        }
    }
}
