use std::{borrow::Cow, str};

use aya_log_common::DisplayHint;

/// A parsed formatting parameter (contents of `{` `}` block).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameter {
    /// The display hint, e.g. ':ipv4', ':IPv4'.
    pub hint: DisplayHint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fragment<'a> {
    /// A literal string (eg. `"literal "` in `"literal {}"`).
    Literal(Cow<'a, str>),

    /// A format parameter.
    Parameter(Parameter),
}

fn push_literal<'a>(
    frag: &mut Vec<Fragment<'a>>,
    unescaped_literal: &'a str,
) -> Result<(), Cow<'static, str>> {
    // Replace `{{` with `{` and `}}` with `}`. Single braces are errors.

    // Scan for single braces first. The rest is trivial.
    let mut last_open = false;
    let mut last_close = false;
    for c in unescaped_literal.chars() {
        match c {
            '{' => last_open = !last_open,
            '}' => last_close = !last_close,
            _ => {
                if last_open {
                    return Err("unmatched `{` in format string".into());
                }
                if last_close {
                    return Err("unmatched `}` in format string".into());
                }
            }
        }
    }

    // Handle trailing unescaped `{` or `}`.
    if last_open {
        return Err("unmatched `{` in format string".into());
    }
    if last_close {
        return Err("unmatched `}` in format string".into());
    }

    // FIXME: This always allocates a `String`, so the `Cow` is useless.
    let literal = unescaped_literal.replace("{{", "{").replace("}}", "}");
    frag.push(Fragment::Literal(literal.into()));
    Ok(())
}

/// Parses the display hint (e.g. the `ipv4` in `{:ipv4}`).
fn parse_display_hint(s: &str) -> Result<DisplayHint, Cow<'static, str>> {
    Ok(match s {
        "x" => DisplayHint::LowerHex,
        "X" => DisplayHint::UpperHex,
        "ipv4" => DisplayHint::IPv4,
        "IPv4" => DisplayHint::IPv4,
        "ipv6" => DisplayHint::IPv6,
        "IPv6" => DisplayHint::IPv6,
        _ => return Err(format!("unknown display hint: {:?}", s).into()),
    })
}

/// Parse `Param` from `&str`
///
/// * example `input`: `:hint` (note: no curly braces)
fn parse_param(mut input: &str) -> Result<Parameter, Cow<'static, str>> {
    const HINT_PREFIX: &str = ":";

    // Then, optional hint
    let mut hint = DisplayHint::Default;

    if input.starts_with(HINT_PREFIX) {
        // skip the prefix
        input = &input[HINT_PREFIX.len()..];
        if input.is_empty() {
            return Err("malformed format string (missing display hint after ':')".into());
        }

        hint = parse_display_hint(input)?;
    } else if !input.is_empty() {
        return Err(format!("unexpected content {:?} in format string", input).into());
    }

    Ok(Parameter { hint })
}

pub fn parse<'a>(format_string: &'a str) -> Result<Vec<Fragment<'a>>, Cow<'static, str>> {
    let mut fragments = Vec::new();

    // Index after the `}` of the last format specifier.
    let mut end_pos = 0;

    let mut chars = format_string.char_indices();
    while let Some((brace_pos, ch)) = chars.next() {
        if ch != '{' {
            // Part of a literal fragment.
            continue;
        }

        // Peek at the next char.
        if chars.as_str().starts_with('{') {
            // Escaped `{{`, also part of a literal fragment.
            chars.next();
            continue;
        }

        if brace_pos > end_pos {
            // There's a literal fragment with at least 1 character before this
            // parameter fragment.
            let unescaped_literal = &format_string[end_pos..brace_pos];
            push_literal(&mut fragments, unescaped_literal)?;
        }

        // Else, this is a format specifier. It ends at the next `}`.
        let len = chars
            .as_str()
            .find('}')
            .ok_or("missing `}` in format string")?;
        end_pos = brace_pos + 1 + len + 1;

        // Parse the contents inside the braces.
        let param_str = &format_string[brace_pos + 1..][..len];
        let param = parse_param(param_str)?;
        fragments.push(Fragment::Parameter(param));
    }

    // Trailing literal.
    if end_pos != format_string.len() {
        push_literal(&mut fragments, &format_string[end_pos..])?;
    }

    Ok(fragments)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse() {
        assert_eq!(
            parse("foo {} bar {:x} test {:X} ayy {:ipv4} lmao {:IPv4} hello {:ipv6} world {:IPv6}"),
            Ok(vec![
                Fragment::Literal("foo ".into()),
                Fragment::Parameter(Parameter {
                    hint: DisplayHint::Default
                }),
                Fragment::Literal(" bar ".into()),
                Fragment::Parameter(Parameter {
                    hint: DisplayHint::LowerHex
                }),
                Fragment::Literal(" test ".into()),
                Fragment::Parameter(Parameter {
                    hint: DisplayHint::UpperHex
                }),
                Fragment::Literal(" ayy ".into()),
                Fragment::Parameter(Parameter {
                    hint: DisplayHint::IPv4
                }),
                Fragment::Literal(" lmao ".into()),
                Fragment::Parameter(Parameter {
                    hint: DisplayHint::IPv4
                }),
                Fragment::Literal(" hello ".into()),
                Fragment::Parameter(Parameter {
                    hint: DisplayHint::IPv6
                }),
                Fragment::Literal(" world ".into()),
                Fragment::Parameter(Parameter {
                    hint: DisplayHint::IPv6
                }),
            ])
        );
    }
}
