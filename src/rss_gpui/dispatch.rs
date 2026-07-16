use std::collections::BTreeMap;

use super::model::ScriptError;

/// Compiles declarative `ui::on(event, || { ... });` blocks into an event table.
///
/// The render source contains only UI declarations. Each event source combines the
/// same declarations with exactly one handler body immediately before `ui::finish()`.
/// The runtime selects an event source by name through this table.
#[derive(Clone, Debug)]
pub struct DispatchProgram {
    render_source: String,
    event_sources: BTreeMap<String, String>,
}

impl DispatchProgram {
    pub fn parse(source: impl Into<String>) -> Result<Self, ScriptError> {
        let source = source.into();
        let handlers = parse_handlers(&source)?;
        if handlers.is_empty() {
            return Ok(Self {
                render_source: source,
                event_sources: BTreeMap::new(),
            });
        }

        let render_source = remove_handler_statements(&source, &handlers);
        let finish_start = find_finish_call(&render_source)
            .ok_or_else(|| ScriptError::new("ui::on handlers require ui::finish()"))?;
        let mut event_sources = BTreeMap::new();
        for handler in handlers {
            let event_source = format!(
                "{}\n{}\n{}",
                &render_source[..finish_start],
                handler.body,
                &render_source[finish_start..]
            );
            event_sources.insert(handler.event, event_source);
        }

        Ok(Self {
            render_source,
            event_sources,
        })
    }

    pub fn source_for(&self, event_name: &str) -> &str {
        self.event_sources
            .get(event_name)
            .map(String::as_str)
            .unwrap_or(&self.render_source)
    }

    pub fn render_source(&self) -> &str {
        &self.render_source
    }
}

#[derive(Debug)]
struct Handler<'source> {
    event: String,
    body: &'source str,
    statement_range: std::ops::Range<usize>,
}

fn parse_handlers(source: &str) -> Result<Vec<Handler<'_>>, ScriptError> {
    let mut handlers = Vec::new();
    let mut cursor = 0;
    while let Some(start) = find_code_token(source, "ui::on", cursor) {
        let next = source.as_bytes().get(start + "ui::on".len());
        if next.is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_') {
            cursor = start + "ui::on".len();
            continue;
        }
        let handler = parse_handler(source, start)?;
        if handlers
            .iter()
            .any(|existing: &Handler<'_>| existing.event == handler.event)
        {
            return Err(ScriptError::new(format!(
                "ui::on event '{}' is registered more than once",
                handler.event
            )));
        }
        cursor = handler.statement_range.end;
        handlers.push(handler);
    }
    Ok(handlers)
}

fn parse_handler<'source>(
    source: &'source str,
    start: usize,
) -> Result<Handler<'source>, ScriptError> {
    let mut cursor = start + "ui::on".len();
    expect_byte(source, &mut cursor, b'(', "expected '(' after ui::on")?;
    let event = parse_string(source, &mut cursor, "ui::on event must be a string")?;
    if event.is_empty() {
        return Err(ScriptError::new("ui::on event must not be empty"));
    }
    expect_byte(source, &mut cursor, b',', "expected ',' after ui::on event")?;
    skip_space(source, &mut cursor);
    if !source[cursor..].starts_with("||") {
        return Err(ScriptError::new(
            "ui::on requires a zero-argument handler closure",
        ));
    }
    cursor += 2;
    skip_space(source, &mut cursor);
    expect_byte(source, &mut cursor, b'{', "expected '{' for ui::on handler")?;
    let body_start = cursor;
    let body_end = find_matching_brace(source, body_start)?;
    cursor = body_end + 1;
    expect_byte(
        source,
        &mut cursor,
        b')',
        "expected ')' after ui::on handler",
    )?;
    expect_byte(
        source,
        &mut cursor,
        b';',
        "expected ';' after ui::on handler",
    )?;

    Ok(Handler {
        event,
        body: &source[body_start..body_end],
        statement_range: start..cursor,
    })
}

fn remove_handler_statements(source: &str, handlers: &[Handler<'_>]) -> String {
    let mut render_source = source.to_owned();
    for handler in handlers.iter().rev() {
        render_source.replace_range(handler.statement_range.clone(), "");
    }
    render_source
}

fn find_finish_call(source: &str) -> Option<usize> {
    let mut cursor = 0;
    let mut last = None;
    while let Some(start) = find_code_token(source, "ui::finish", cursor) {
        let mut end = start + "ui::finish".len();
        skip_space(source, &mut end);
        if source.as_bytes().get(end) != Some(&b'(') {
            cursor = end;
            continue;
        }
        end += 1;
        skip_space(source, &mut end);
        if source.as_bytes().get(end) != Some(&b')') {
            cursor = end;
            continue;
        }
        end += 1;
        skip_space(source, &mut end);
        if source.as_bytes().get(end) == Some(&b';') {
            last = Some(start);
        }
        cursor = end;
    }
    last
}

fn find_code_token(source: &str, needle: &str, mut cursor: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    while cursor < bytes.len() {
        if source[cursor..].starts_with(needle) {
            return Some(cursor);
        }
        match bytes[cursor] {
            b'"' => cursor = skip_string(source, cursor).ok()?,
            b'/' if bytes.get(cursor + 1) == Some(&b'/') => {
                cursor = source[cursor..]
                    .find('\n')
                    .map(|offset| cursor + offset + 1)
                    .unwrap_or(bytes.len());
            }
            b'/' if bytes.get(cursor + 1) == Some(&b'*') => {
                let offset = source[cursor + 2..].find("*/")?;
                cursor += offset + 4;
            }
            _ => cursor += 1,
        }
    }
    None
}

fn find_matching_brace(source: &str, mut cursor: usize) -> Result<usize, ScriptError> {
    let bytes = source.as_bytes();
    let mut depth = 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' => cursor = skip_string(source, cursor)?,
            b'/' if bytes.get(cursor + 1) == Some(&b'/') => {
                cursor = source[cursor..]
                    .find('\n')
                    .map(|offset| cursor + offset + 1)
                    .unwrap_or(bytes.len());
            }
            b'/' if bytes.get(cursor + 1) == Some(&b'*') => {
                let Some(offset) = source[cursor + 2..].find("*/") else {
                    return Err(ScriptError::new(
                        "unterminated block comment in ui::on handler",
                    ));
                };
                cursor += offset + 4;
            }
            b'{' => {
                depth += 1;
                cursor += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(cursor);
                }
                cursor += 1;
            }
            _ => cursor += 1,
        }
    }
    Err(ScriptError::new("unterminated ui::on handler body"))
}

fn parse_string(source: &str, cursor: &mut usize, message: &str) -> Result<String, ScriptError> {
    skip_space(source, cursor);
    if source.as_bytes().get(*cursor) != Some(&b'"') {
        return Err(ScriptError::new(message));
    }
    let start = *cursor;
    let end = skip_string(source, start)?;
    let literal = &source[start + 1..end - 1];
    *cursor = end;
    Ok(unescape(literal))
}

fn skip_string(source: &str, start: usize) -> Result<usize, ScriptError> {
    let bytes = source.as_bytes();
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => cursor += 2,
            b'"' => return Ok(cursor + 1),
            _ => cursor += 1,
        }
    }
    Err(ScriptError::new("unterminated string literal"))
}

fn unescape(literal: &str) -> String {
    let mut out = String::new();
    let mut escaped = false;
    for character in literal.chars() {
        if escaped {
            out.push(match character {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            out.push(character);
        }
    }
    if escaped {
        out.push('\\');
    }
    out
}

fn expect_byte(
    source: &str,
    cursor: &mut usize,
    expected: u8,
    message: &str,
) -> Result<(), ScriptError> {
    skip_space(source, cursor);
    if source.as_bytes().get(*cursor) != Some(&expected) {
        return Err(ScriptError::new(message));
    }
    *cursor += 1;
    Ok(())
}

fn skip_space(source: &str, cursor: &mut usize) {
    while source
        .as_bytes()
        .get(*cursor)
        .is_some_and(u8::is_ascii_whitespace)
    {
        *cursor += 1;
    }
}
