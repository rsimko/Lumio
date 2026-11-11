use chrono::{Local, TimeZone};

use super::types::CaptureInfo;

pub const TOKEN_DESCRIPTIONS: &[(&str, &str)] = &[
    ("YYYY", "Four-digit year"),
    ("MM", "Two-digit month"),
    ("DD", "Two-digit day"),
    ("hh", "Two-digit hour (24h)"),
    ("mm", "Two-digit minute"),
    ("ss", "Two-digit second"),
    ("job_code", "Job code from the ingest form"),
    ("custom_text", "Custom text from the ingest form"),
    (
        "default_filename",
        "Original file name without its extension",
    ),
    ("counter", "Per-photo counter"),
];

const CAMERA_TOKENS: &[&str] = &[
    "make",
    "model",
    "camera",
    "camera_make",
    "camera_model",
    "lens",
    "lens_make",
    "lens_model",
    "iso",
    "focal_length",
    "aperture",
    "shutter_speed",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateKind {
    Folder,
    Filename,
    Text,
}

/// One token in a parsed template. Literals keep the exact text the user typed.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece<'a> {
    Literal(&'a str),
    Token(&'a str),
}

fn parse<'a>(template: &'a str) -> Result<Vec<Piece<'a>>, String> {
    let mut pieces = Vec::new();
    let mut rest = template;
    loop {
        let Some(open) = rest.find('{') else {
            if rest.contains('}') {
                return Err("Unmatched '}' in template.".into());
            }
            if !rest.is_empty() {
                pieces.push(Piece::Literal(rest));
            }
            break;
        };
        if rest[..open].contains('}') {
            return Err("Unmatched '}' in template.".into());
        }
        if open > 0 {
            pieces.push(Piece::Literal(&rest[..open]));
        }
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            return Err("Unclosed '{' in template.".into());
        };
        let token = &after[..close];
        if token.is_empty() {
            return Err("Empty token '{}' in template.".into());
        }
        if token.contains('{') {
            return Err("Nested '{' in template.".into());
        }
        pieces.push(Piece::Token(token));
        rest = &after[close + 1..];
    }
    Ok(pieces)
}

fn check_token(token: &str) -> Result<(), String> {
    if matches!(
        token,
        "YYYY"
            | "MM"
            | "DD"
            | "hh"
            | "mm"
            | "ss"
            | "job_code"
            | "custom_text"
            | "default_filename"
            | "counter"
    ) {
        return Ok(());
    }
    if token == "original" {
        return Err(
            "'{original}' is not used in Lumio. Use '{default_filename}' for the original name."
                .into(),
        );
    }
    if CAMERA_TOKENS.contains(&token.to_ascii_lowercase().as_str()) {
        return Err(format!(
            "'{{{token}}}' is a camera token. Lumio deliberately excludes camera make/model tokens."
        ));
    }
    Err(format!("Unknown template token '{{{token}}}'."))
}

/// Validate a template without rendering it. Returns every problem found so the
/// editor can show all mistakes at once.
pub fn validate_template(template: &str, kind: TemplateKind) -> Vec<String> {
    let mut problems = Vec::new();
    let pieces = match parse(template) {
        Ok(pieces) => pieces,
        Err(error) => {
            problems.push(error);
            return problems;
        }
    };

    for piece in &pieces {
        match piece {
            Piece::Token(token) => {
                if let Err(error) = check_token(token) {
                    problems.push(error);
                }
            }
            Piece::Literal(text) => {
                if kind == TemplateKind::Filename && text.contains('/') {
                    problems.push("The filename template cannot contain '/'.".into());
                }
                if kind == TemplateKind::Text {
                    // Metadata values may contain any printable characters. Only
                    // forbid control characters so the result stays valid text.
                    if text.chars().any(char::is_control) {
                        problems.push("The template cannot contain control characters.".into());
                    }
                    continue;
                }
                if text.contains('\\') {
                    problems.push("The template cannot contain '\\'.".into());
                }
                for character in text.chars() {
                    if character.is_control() {
                        problems.push("The template cannot contain control characters.".into());
                        break;
                    }
                    if matches!(character, ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                        problems.push(format!(
                            "The character '{character}' is not allowed in file names."
                        ));
                    }
                }
            }
        }
    }

    if kind == TemplateKind::Folder {
        let trimmed = template.trim();
        if trimmed.starts_with('/') {
            problems.push("The folder template must be relative to the destination.".into());
        }
        if trimmed.split('/').any(|segment| segment.trim() == "..") {
            problems.push("The folder template cannot contain '..'.".into());
        }
        if trimmed.split('/').any(|segment| segment.trim() == ".") {
            problems.push("The folder template cannot contain '.' segments.".into());
        }
    }

    if kind == TemplateKind::Filename {
        let trimmed = template.trim();
        if trimmed.is_empty() {
            problems.push("Enter a filename template.".into());
        }
        if trimmed == "." || trimmed == ".." {
            problems.push("The filename template must produce a file name.".into());
        }
    }

    problems.sort();
    problems.dedup();
    problems
}

pub fn template_uses_token(template: &str, needle: &str) -> bool {
    parse(template)
        .map(|pieces| {
            pieces
                .iter()
                .any(|piece| matches!(piece, Piece::Token(token) if *token == needle))
        })
        .unwrap_or(false)
}

pub fn validate_required_inputs(
    template: &str,
    kind: TemplateKind,
    job_code: &str,
    custom_text: &str,
) -> Vec<String> {
    let mut problems = validate_template(template, kind);
    if template_uses_token(template, "job_code") && job_code.trim().is_empty() {
        problems.push("This template uses '{job_code}' but Job code is empty.".into());
    }
    if template_uses_token(template, "custom_text") && custom_text.trim().is_empty() {
        problems.push("This template uses '{custom_text}' but Custom text is empty.".into());
    }
    problems
}

pub struct TemplateContext<'a> {
    pub capture: &'a CaptureInfo,
    pub job_code: &'a str,
    pub custom_text: &'a str,
    pub default_filename: &'a str,
    pub counter: u32,
    pub counter_padding: u32,
}

impl TemplateContext<'_> {
    fn date_tokens(&self) -> Option<chrono::DateTime<Local>> {
        Local
            .timestamp_millis_opt(self.capture.timestamp_ms)
            .single()
    }

    fn token(&self, token: &str) -> Result<String, String> {
        check_token(token)?;
        if let Some(date) = self.date_tokens() {
            let value = match token {
                "YYYY" => Some(date.format("%Y").to_string()),
                "MM" => Some(date.format("%m").to_string()),
                "DD" => Some(date.format("%d").to_string()),
                "hh" => Some(date.format("%H").to_string()),
                "mm" => Some(date.format("%M").to_string()),
                "ss" => Some(date.format("%S").to_string()),
                _ => None,
            };
            if let Some(value) = value {
                return Ok(value);
            }
        }

        match token {
            "YYYY" | "MM" | "DD" | "hh" | "mm" | "ss" => {
                Err("The capture time could not be formatted.".into())
            }
            "job_code" => {
                let value = self.job_code.trim();
                if value.is_empty() {
                    Err("This template uses '{job_code}' but Job code is empty.".into())
                } else {
                    Ok(value.to_string())
                }
            }
            "custom_text" => {
                let value = self.custom_text.trim();
                if value.is_empty() {
                    Err("This template uses '{custom_text}' but Custom text is empty.".into())
                } else {
                    Ok(value.to_string())
                }
            }
            "default_filename" => {
                if self.default_filename.trim().is_empty() {
                    Err("The original file name is empty.".into())
                } else {
                    Ok(self.default_filename.to_string())
                }
            }
            "counter" => Ok(format!(
                "{:0width$}",
                self.counter,
                width = self.counter_padding.min(12) as usize
            )),
            _ => Err(format!("Unknown template token '{{{token}}}'.")),
        }
    }
}

pub fn render(template: &str, context: &TemplateContext<'_>) -> Result<String, String> {
    let pieces = parse(template)?;
    let mut output = String::new();
    for piece in pieces {
        match piece {
            Piece::Literal(text) => output.push_str(text),
            Piece::Token(token) => output.push_str(&context.token(token)?),
        }
    }
    Ok(output)
}

/// Render an IPTC/text template. Text values may contain any characters.
pub fn render_text(template: &str, context: &TemplateContext<'_>) -> Result<String, String> {
    render(template, context)
}

fn is_reserved_name(name: &str) -> bool {
    let base = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && base[3..]
                .chars()
                .all(|character| character.is_ascii_digit()))
}

/// Validate the rendered file name. This is separate from template validation
/// because job codes and custom text can inject unsafe characters.
pub fn validate_rendered_filename(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("The filename template produced an empty file name.".into());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("The filename template produced a reserved name.".into());
    }
    if trimmed.len() > 240 {
        return Err("The generated file name is too long.".into());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("The generated file name contains a path separator.".into());
    }
    for character in trimmed.chars() {
        if character.is_control() {
            return Err("The generated file name contains a control character.".into());
        }
        if matches!(character, ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            return Err(format!(
                "The generated file name contains '{character}', which is not allowed."
            ));
        }
    }
    if trimmed.ends_with('.') || trimmed.ends_with(' ') {
        return Err("The generated file name cannot end with a dot or space.".into());
    }
    if is_reserved_name(trimmed) {
        return Err("The generated file name is reserved by the operating system.".into());
    }
    Ok(trimmed.to_string())
}

/// Render and validate a destination folder template into relative segments.
pub fn render_folder_segments(
    template: &str,
    context: &TemplateContext<'_>,
) -> Result<Vec<String>, String> {
    let rendered = render(template, context)?;
    let mut segments = Vec::new();
    for raw in rendered.split('/') {
        let segment = raw.trim();
        if segment.is_empty() {
            continue;
        }
        if segment == "." || segment == ".." {
            return Err("The generated folder contains a reserved path segment.".into());
        }
        if segment.len() > 200 {
            return Err("A generated folder name is too long.".into());
        }
        for character in segment.chars() {
            if character.is_control() {
                return Err("A generated folder name contains a control character.".into());
            }
            if matches!(character, '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                return Err(format!(
                    "A generated folder name contains '{character}', which is not allowed."
                ));
            }
        }
        if segment.ends_with('.') || segment.ends_with(' ') {
            return Err("A generated folder name cannot end with a dot or space.".into());
        }
        segments.push(segment.to_string());
    }
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::types::CaptureSource;

    fn context<'a>(capture: &'a CaptureInfo, default_filename: &'a str) -> TemplateContext<'a> {
        TemplateContext {
            capture,
            job_code: "JOB42",
            custom_text: "Editorial",
            default_filename,
            counter: 7,
            counter_padding: 4,
        }
    }

    fn capture() -> CaptureInfo {
        let date = Local.with_ymd_and_hms(2024, 5, 3, 14, 22, 9).unwrap();
        CaptureInfo {
            display: date.format("%Y-%m-%d %H:%M:%S").to_string(),
            timestamp_ms: date.timestamp_millis(),
            source: CaptureSource::RawExif,
            warning: None,
        }
    }

    #[test]
    fn renders_date_and_value_tokens() {
        let capture = capture();
        let context = context(&capture, "palawan-112");
        assert_eq!(
            render("{YYYY}-{MM}-{DD}_{hh}{mm}{ss}", &context).unwrap(),
            "2024-05-03_142209"
        );
        assert_eq!(
            render(
                "{job_code}_{custom_text}_{default_filename}_{counter}",
                &context
            )
            .unwrap(),
            "JOB42_Editorial_palawan-112_0007"
        );
    }

    #[test]
    fn counter_padding_and_start_are_configurable() {
        let capture = capture();
        let mut context = context(&capture, "photo");
        context.counter = 42;
        context.counter_padding = 2;
        assert_eq!(render("{counter}", &context).unwrap(), "42");
        context.counter_padding = 0;
        assert_eq!(render("{counter}", &context).unwrap(), "42");
    }

    #[test]
    fn rejects_unsupported_tokens_and_camera_tokens() {
        assert!(
            validate_template("{original}", TemplateKind::Filename)
                .iter()
                .any(|message| message.contains("{default_filename}"))
        );
        assert!(
            validate_template("{make}_{model}", TemplateKind::Filename)
                .iter()
                .any(|message| message.contains("camera token"))
        );
        assert!(
            validate_template("{not_a_token}", TemplateKind::Filename)
                .iter()
                .any(|message| message.contains("Unknown"))
        );
        assert!(validate_template("{YYYY", TemplateKind::Filename).len() == 1);
        assert!(validate_template("photo}.jpg", TemplateKind::Filename).len() == 1);
    }

    #[test]
    fn folder_templates_build_segments_and_reject_escapes() {
        let capture = capture();
        let context = context(&capture, "photo");
        assert_eq!(
            render_folder_segments("{YYYY}/{MM}/Shoot", &context).unwrap(),
            vec!["2024", "05", "Shoot"]
        );
        assert!(render_folder_segments("../outside", &context).is_err());
        assert!(
            validate_template("/absolute", TemplateKind::Folder)
                .iter()
                .any(|message| message.contains("relative"))
        );
    }

    #[test]
    fn filename_validation_rejects_filesystem_characters() {
        assert!(validate_rendered_filename("photo.jpg").is_ok());
        assert!(validate_rendered_filename("../photo.jpg").is_err());
        assert!(validate_rendered_filename("a:b.jpg").is_err());
        assert!(validate_rendered_filename("photo.jpg ").is_ok());
        assert!(validate_rendered_filename("photo.").is_err());
        assert!(validate_rendered_filename("CON").is_err());
        assert!(validate_template("a/b", TemplateKind::Filename).len() == 1);
    }

    #[test]
    fn text_templates_allow_metadata_punctuation() {
        assert!(validate_template("Morning: blue hour?", TemplateKind::Text).is_empty());
        assert!(validate_template("Fjord / Morning", TemplateKind::Text).is_empty());
    }

    #[test]
    fn required_inputs_are_reported() {
        assert!(
            validate_required_inputs("{job_code}_{YYYY}", TemplateKind::Filename, "", "")
                .iter()
                .any(|message| message.contains("Job code"))
        );
        assert!(
            validate_required_inputs("{custom_text}", TemplateKind::Filename, "job", "")
                .iter()
                .any(|message| message.contains("Custom text"))
        );
    }
}
