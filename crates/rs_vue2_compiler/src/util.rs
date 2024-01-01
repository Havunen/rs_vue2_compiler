use crate::MODIFIER_RE;
use rs_html_parser_tokenizer_tokens::QuoteType;
use rs_html_parser_tokens::Token;
use std::collections::BTreeMap;

pub fn has_attribute(token: &Token, str: &str) -> bool {
    if let Some(attrs) = &token.attrs {
        return match attrs.get(str) {
            Some(_) => true,
            None => false,
        };
    }

    false
}

pub fn get_attribute<'a>(token: &'a Token, str: &str) -> &'a Option<(Box<str>, QuoteType)> {
    if let Some(attrs) = &token.attrs {
        return attrs.get(str).unwrap_or_else(|| &None);
    }

    &None
}

pub fn prepend_modifier_marker(symbol: char, name: &str, dynamic: bool) -> String {
    return if dynamic {
        format!("_p({}, \"{}\")", name, symbol)
    } else {
        format!("{}{}", symbol, name)
    };
}

pub fn modifier_regex_replace_all_matches(input: &str) -> String {
    // This method simulates the original regex behavior
    // "\.[^.\]]+(?=[^\]]*$)"
    let mut replaced_string = input.to_string();

    for captures in MODIFIER_RE.captures_iter(input) {
        let matched_string = &captures[0];
        if !matched_string.contains(']') {
            replaced_string = replaced_string.replace(matched_string, "");
        }
    }

    replaced_string
}

pub fn parse_style_text(css_text: &str) -> BTreeMap<String, String> {
    let mut res = BTreeMap::new();

    for item in css_text.split(";") {
        if !item.is_empty() {
            let tmp: Vec<&str> = item.split(":").collect();
            if tmp.len() > 1 {
                res.insert(tmp[0].trim().to_string(), tmp[1].trim().to_string());
            }
        }
    }

    res
}
