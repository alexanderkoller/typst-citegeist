use biblatex::{ParseError, ParseErrorKind, Token};

pub(crate) fn format_parse_error(
    source: &str,
    source_name: Option<&str>,
    err: &ParseError,
) -> String {
    let offset = err.span.start.min(source.len());
    let (line, column) = line_column(source, offset);
    let mut message = match source_name {
        Some(name) => format!("failed to parse bibliography in {name}"),
        None => "failed to parse bibliography".to_string(),
    };

    message.push_str(&format!("\n  {}", err.kind));
    message.push_str(&format!(
        "\n  at line {line}, column {column}, byte {}",
        err.span.start
    ));
    if err.span.end != err.span.start {
        message.push_str(&format!("-{}", err.span.end));
    }

    if let Some(context) = nearest_entry_context(source, offset) {
        message.push_str(&format!("\n  while parsing {context}"));
    }

    if let Some(excerpt) = excerpt_with_caret(source, offset) {
        message.push('\n');
        message.push_str(&excerpt);
    }

    if let Some(hint) = parse_error_hint(&err.kind) {
        message.push_str(&format!("\nhint: {hint}"));
    }

    message
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut line_start = 0;

    for (idx, ch) in source.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = idx + ch.len_utf8();
        }
    }

    let column = source[line_start..offset.min(source.len())].chars().count() + 1;
    (line, column)
}

fn excerpt_with_caret(source: &str, offset: usize) -> Option<String> {
    if source.is_empty() {
        return None;
    }

    let offset = offset.min(source.len());
    let line_start = source[..offset].rfind('\n').map_or(0, |idx| idx + 1);
    let line_end = source[offset..]
        .find('\n')
        .map_or(source.len(), |idx| offset + idx);
    let line_number = source[..line_start]
        .chars()
        .filter(|&ch| ch == '\n')
        .count()
        + 1;
    let line_text = &source[line_start..line_end];
    let caret_column = source[line_start..offset.min(line_end)].chars().count();

    Some(format!(
        "{line_number:>4} | {line_text}\n     | {}^",
        " ".repeat(caret_column)
    ))
}

fn nearest_entry_context(source: &str, offset: usize) -> Option<String> {
    let limit = offset.min(source.len());
    let mut last_context = None;
    let mut depth = 0usize;
    let mut in_quote = false;
    let mut escaped = false;
    let mut in_comment = false;

    for (idx, ch) in source.char_indices() {
        if idx >= limit {
            break;
        }

        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }

        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' && (in_quote || depth > 0) {
            escaped = true;
            continue;
        }

        if ch == '%' && !in_quote {
            in_comment = true;
            continue;
        }

        if ch == '"' && depth > 0 {
            in_quote = !in_quote;
            continue;
        }

        if in_quote {
            continue;
        }

        if ch == '@' && depth == 0 {
            if let Some(context) = parse_entry_header(source, idx) {
                last_context = Some(context);
            }
            continue;
        }

        match ch {
            '{' | '(' => depth += 1,
            '}' | ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }

    last_context
}

fn parse_entry_header(source: &str, at: usize) -> Option<String> {
    let mut rest = source.get(at + 1..)?;
    rest = rest.trim_start();

    let entry_type_len = rest
        .char_indices()
        .find(|(_, ch)| !(ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-'))
        .map_or(rest.len(), |(idx, _)| idx);
    let entry_type = rest[..entry_type_len].trim();
    if entry_type.is_empty() {
        return None;
    }

    rest = rest[entry_type_len..].trim_start();
    if !rest.starts_with('{') && !rest.starts_with('(') {
        return None;
    }
    rest = &rest[1..];

    let key_len = rest
        .char_indices()
        .find(|(_, ch)| *ch == ',' || ch.is_whitespace() || *ch == '}' || *ch == ')')
        .map_or(rest.len(), |(idx, _)| idx);
    let key = rest[..key_len].trim();
    if key.is_empty() {
        Some(format!("@{entry_type}"))
    } else {
        Some(format!("@{entry_type}{{{key}}}"))
    }
}

fn parse_error_hint(kind: &ParseErrorKind) -> Option<&'static str> {
    match kind {
        ParseErrorKind::Expected(Token::Comma) => Some(
            "BibTeX fields must be separated by commas; check the previous field or closing brace.",
        ),
        ParseErrorKind::UnexpectedEof => Some(
            "The bibliography ended before the current entry was complete; check for an unclosed brace, quote, or field value.",
        ),
        ParseErrorKind::DuplicateKey(_) => Some(
            "Citation keys must be unique unless you call load-bibliography with on-duplicate: \"keep-first\" or \"keep-last\".",
        ),
        ParseErrorKind::UnknownAbbreviation(_) => Some(
            "Define the abbreviation with @string before using it, or wrap the value in braces or quotes.",
        ),
        _ => None,
    }
}
