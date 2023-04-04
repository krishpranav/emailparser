use charset::Charset;

use crate::find_from;

pub enum HeaderToken<'a> {
    Text(&'a str),
    Whitespace(&'a str),
    Newline(Option<String>),
    DecodedWord(String),
}

fn is_boundary(line: &str, ix: Option<usize>) -> bool {
    ix.and_then(|v| line.chars().nth(v))
        .map(|c| {
            c.is_whitespace()
                || c == '"'
                || c == '('
                || c == ')'
                || c == '<'
                || c == '>'
                || c == ','
        })
        .unwrap_or(true)
}

fn decode_word(encoded: &str) -> Option<String> {
    let ix_delim1 = encoded.find('?')?;
    let ix_delim2 = find_from(encoded, ix_delim1 + 1, "?")?;

    let charset = &encoded[0..ix_delim1];
    let transfer_coding = &encoded[ix_delim1 + 1..ix_delim2];
    let input = &encoded[ix_delim2 + 1..];

    let decoded = match transfer_coding {
        "B" | "b" => data_encoding::BASE64_MIME.decode(input.as_bytes()).ok()?,
        "Q" | "q" => {
            let to_decode = input.replace('_', " ");
            let trimmed = to_decode.trim_end();
            let mut d = quoted_printable::decode(trimmed, quoted_printable::ParseMode::Robust);
            if d.is_ok() && to_decode.len() != trimmed.len() {
                d.as_mut()
                    .unwrap()
                    .extend_from_slice(to_decode[trimmed.len()..].as_bytes());
            }
            d.ok()?
        }
        _ => return None,
    };
    let charset = Charset::for_label_no_replacement(charset.as_bytes())?;
    let (cow, _) = charset.decode_without_bom_handling(&decoded);
    Some(cow.into_owned())
}

fn tokenize_header_line(line: &str) -> Vec<HeaderToken> {
    fn maybe_whitespace(text: &str) -> HeaderToken {
        if text.trim_end().is_empty() {
            HeaderToken::Whitespace(text)
        } else {
            HeaderToken::Text(text)
        }
    }

    let mut result = Vec::new();
    let mut ix_search = 0;
    loop {
        match find_from(line, ix_search, "=?") {
            Some(v) => {
                let ix_begin = v + 2;
                if !is_boundary(line, ix_begin.checked_sub(3)) {
                    result.push(HeaderToken::Text(&line[ix_search..ix_begin]));
                    ix_search = ix_begin;
                    continue;
                }
                result.push(maybe_whitespace(&line[ix_search..ix_begin - 2]));
                let mut ix_end_search = ix_begin;
                loop {
                    match find_from(line, ix_end_search, "?=") {
                        Some(ix_end) => {
                            if !is_boundary(line, ix_end.checked_add(2)) {
                                ix_end_search = ix_end + 2;
                                continue;
                            }
                            match decode_word(&line[ix_begin..ix_end]) {
                                Some(v) => result.push(HeaderToken::DecodedWord(v)),
                                None => {
                                    result.push(HeaderToken::Text(&line[ix_begin - 2..ix_end + 2]));
                                }
                            };
                            ix_search = ix_end;
                        }
                        None => {
                            result.push(HeaderToken::Text("=?"));
                            ix_search = ix_begin - 2;
                        }
                    };
                    break;
                }
                ix_search += 2;
                continue;
            }
            None => {
                result.push(maybe_whitespace(&line[ix_search..]));
                break;
            }
        };
    }
    result
}

fn tokenize_header(value: &str) -> Vec<HeaderToken> {
    let mut tokens = Vec::new();
    let mut lines = value.lines();
    let mut first = true;
    while let Some(line) = lines.next().map(str::trim_start) {
        if first {
            first = false;
        } else {
            tokens.push(HeaderToken::Newline(None));
        }
        let mut line_tokens = tokenize_header_line(line);
        tokens.append(&mut line_tokens);
    }
    tokens
}

fn normalize_header_whitespace(tokens: Vec<HeaderToken>) -> Vec<HeaderToken> {
    let mut result = Vec::<HeaderToken>::new();

    let mut saved_token = None;
    for tok in tokens {
        match &tok {
            HeaderToken::Text(_) => {
                if let Some(HeaderToken::Whitespace(_)) = &saved_token {
                    result.push(saved_token.unwrap());
                } else if let Some(HeaderToken::Newline(Some(_))) = &saved_token {
                    result.push(saved_token.unwrap());
                }
                result.push(tok);
                saved_token = None;
            }
            HeaderToken::Whitespace(_) => {
                if let Some(HeaderToken::DecodedWord(_)) = saved_token {
                    saved_token = Some(tok);
                } else {
                    result.push(tok);
                    saved_token = None;
                }
            }
            HeaderToken::Newline(_) => {
                if let Some(HeaderToken::Whitespace(ws)) = saved_token {
                    let new_ws = ws.to_owned() + " ";
                    saved_token = Some(HeaderToken::Newline(Some(new_ws)));
                } else if let Some(HeaderToken::DecodedWord(_)) = saved_token {
                    saved_token = Some(HeaderToken::Newline(Some(" ".to_string())));
                } else {
                    result.push(HeaderToken::Newline(Some(" ".to_string())));
                    saved_token = None;
                }
            }
            HeaderToken::DecodedWord(_) => {
                saved_token = Some(HeaderToken::DecodedWord(String::new()));
                result.push(tok);
            }
        }
    }
    result
}

pub fn normalized_tokens(raw_value: &str) -> Vec<HeaderToken> {
    normalize_header_whitespace(tokenize_header(raw_value))
}