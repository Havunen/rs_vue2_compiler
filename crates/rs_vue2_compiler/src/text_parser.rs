use crate::filter_parser::parse_filters;
use lazy_static::lazy_static;
use regex::Regex;
use rs_html_parser_tokenizer_tokens::QuoteType;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref REGEX_CACHE: Mutex<HashMap<(String, String), Arc<Regex>>> =
        Mutex::new(HashMap::new());
    static ref DEFAULT_REGEX: Arc<Regex> = Arc::new(Regex::new(r"\{\{((?:.|\r?\n)+?)}}").unwrap());
}

fn build_regex(delimiters: (String, String)) -> Arc<Regex> {
    let mut cache = REGEX_CACHE.lock().unwrap();
    if let Some(regex) = cache.get(&delimiters) {
        return Arc::clone(regex);
    }
    let (open, close) = &delimiters;
    let regex = Arc::new(
        Regex::new(&format!(
            "{}((?:.|\\n)+?){}",
            regex::escape(&open),
            regex::escape(&close)
        ))
        .unwrap(),
    );
    cache.insert(delimiters, Arc::clone(&regex));

    regex
}

pub fn parse_text(
    text: &str,
    delimiters: Option<(String, String)>,
) -> Option<(String, Vec<String>)> {
    let tag_re = match delimiters {
        Some(delimiters) => build_regex(delimiters),
        None => Arc::clone(&DEFAULT_REGEX),
    };
    if !tag_re.is_match(text) {
        return None;
    }
    let mut tokens = Vec::new();
    let mut raw_tokens = Vec::new();
    let mut last_index = 0;
    for cap in tag_re.captures_iter(text) {
        let index = cap.get(0).unwrap().start();
        if index > last_index {
            let token_value = text[last_index..index].to_string();
            raw_tokens.push(token_value.clone());
            tokens.push(format!(r#""{}""#, token_value));
        }
        let exp = parse_filters(&(
            cap[1].trim().to_string().into_boxed_str(),
            QuoteType::NoValue,
        ));
        tokens.push(format!("_s({})", exp));
        raw_tokens.push(format!("@binding: {}", exp));
        last_index = cap.get(0).unwrap().end();
    }
    if last_index < text.len() {
        let token_value = text[last_index..].to_string();
        raw_tokens.push(token_value.clone());
        tokens.push(format!(r#""{}""#, token_value));
    }
    Some((tokens.join("+"), raw_tokens))
}
