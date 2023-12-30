use crate::MODIFIER_RE;
use rs_html_parser_tokenizer_tokens::QuoteType;
use rs_html_parser_tokens::Token;

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
        return match attrs.get(str) {
            Some(val) => val,
            None => &None,
        };
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
