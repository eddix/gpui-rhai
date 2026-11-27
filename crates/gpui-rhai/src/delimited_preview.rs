//! Bounded, read-only work for attachment previews; invalid data stays text.
use rhai::{Dynamic, Engine, ImmutableString};

const MAX_BYTES: usize = 32_768;
const MAX_ROWS: usize = 128;
const MAX_COLUMNS: usize = 16;

#[derive(Clone, Copy, Eq, PartialEq)]
enum Field {
    Start,
    Plain,
    Quoted,
    Closed,
}

fn finish_field(row: &mut Vec<String>, field: &mut String) -> Option<()> {
    if row.len() == MAX_COLUMNS {
        return None;
    }
    row.push(std::mem::take(field));
    Some(())
}

fn finish_row(
    table: &mut Vec<Vec<String>>,
    row: &mut Vec<String>,
    field: &mut String,
) -> Option<()> {
    finish_field(row, field)?;
    if table.len() == MAX_ROWS
        || table
            .first()
            .is_some_and(|header| header.len() != row.len())
    {
        return None;
    }
    table.push(std::mem::take(row));
    Some(())
}

fn parse(input: &str, delimiter: char) -> Option<Vec<Vec<String>>> {
    if input.is_empty() || input.len() > MAX_BYTES {
        return None;
    }
    let mut table = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut state = Field::Start;
    let mut chars = input
        .strip_prefix('\u{feff}')
        .unwrap_or(input)
        .chars()
        .peekable();
    while let Some(c) = chars.next() {
        if state == Field::Quoted {
            if c != '"' {
                field.push(c);
            } else if chars.next_if_eq(&'"').is_some() {
                field.push('"');
            } else {
                state = Field::Closed;
            }
        } else if c == delimiter {
            finish_field(&mut row, &mut field)?;
            state = Field::Start;
        } else if c == '\n' || c == '\r' {
            if c == '\r' {
                chars.next_if_eq(&'\n');
            }
            finish_row(&mut table, &mut row, &mut field)?;
            state = Field::Start;
        } else if state == Field::Start && c == '"' {
            state = Field::Quoted;
        } else if state == Field::Closed || c == '"' {
            return None;
        } else {
            state = Field::Plain;
            field.push(c);
        }
    }
    if state == Field::Quoted {
        return None;
    }
    if state != Field::Start || !row.is_empty() {
        finish_row(&mut table, &mut row, &mut field)?;
    }
    (!table.is_empty()).then_some(table)
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn(
        "parse_delimited_preview",
        |source: ImmutableString, tab: bool| {
            parse(source.as_str(), if tab { '\t' } else { ',' }).map_or(Dynamic::UNIT, |rows| {
                Dynamic::from_array(
                    rows.into_iter()
                        .map(|row| {
                            Dynamic::from_array(row.into_iter().map(Dynamic::from).collect())
                        })
                        .collect(),
                )
            })
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delimited_preview_preserves_quoted_cells_and_formats() {
        assert_eq!(
            parse(
                "\u{feff}phase,result\r\nreview,\"yes, with \"\"notes\"\"\nnext line\"\r\n",
                ','
            )
            .unwrap(),
            [
                ["phase", "result"],
                ["review", "yes, with \"notes\"\nnext line"]
            ]
        );
        assert_eq!(parse("a\tb\n1\t", '\t').unwrap(), [["a", "b"], ["1", ""]]);
    }

    #[test]
    fn delimited_preview_declines_ambiguous_or_oversized_data_for_plain_fallback() {
        for input in ["a,b\n1", "a,\"unfinished", "a,\"b\"x", "a,z\"z"] {
            assert!(parse(input, ',').is_none());
        }
        assert!(parse(&"a\n".repeat(MAX_ROWS + 1), ',').is_none());
        assert!(parse(&",".repeat(MAX_COLUMNS), ',').is_none());
        assert!(parse(&"a".repeat(MAX_BYTES + 1), ',').is_none());
    }
}
