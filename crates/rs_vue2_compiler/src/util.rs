use std::borrow::Cow;
use rs_html_parser_tokenizer_tokens::QuoteType;
use rs_html_parser_tokens::Token;
use unicase::UniCase;

pub fn has_attribute<'a>(token: &Token, str: &UniCase<&str>) -> bool {
    if let Some(attrs) = &token.attrs {
        return match attrs.get(str) {
            Some(_) => {
                true
            },
            None => {
                false
            }
        }
    }

    false
}

pub fn get_attribute<'a>(token: &'a Token, str: &'a UniCase<&'a str>) -> &'a Option<(Cow<'a, str>, QuoteType)> {
    if let Some(attrs) = &token.attrs {
        return match attrs.get(str) {
            Some(val) => {
                val
            },
            None => &None
        }
    }

    &None
}
