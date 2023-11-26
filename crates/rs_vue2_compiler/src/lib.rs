mod util;

use std::borrow::Cow;
use lazy_static::lazy_static;
use regex::Regex;
use rs_html_parser::{Parser, ParserOptions};
use rs_html_parser_tokenizer::TokenizerOptions;
use rs_html_parser_tokens::Token;
use rs_html_parser_tokens::TokenKind::OpenTag;
use unicase::UniCase;
use crate::util::{get_attribute, has_attribute};

lazy_static! {
    static ref invalidAttributeRE: Regex = Regex::new(r##"/[\s"'<>\/=]/"##).unwrap();
}


// TODO: Move to options
fn warn(message: &str) {
    println!("{}", message)
}

struct CompilerOptions {
    dev: bool
}

struct ElementTags {}

impl ElementTags {
    const TYPE: UniCase<&'static str> = UniCase::new("type");
}

fn is_forbidden_tag(el: &Token) -> bool {
    if &el.kind != &OpenTag {
        return false;
    }

    match &el.data {
        Cow::Borrowed("style") => true,
        Cow::Borrowed("script") => {
            let attr_value = get_attribute(el, &ElementTags::TYPE);

            if let Some((val, _quote)) = attr_value {
                return val == "text/javascript"
            }

            return false;
        }
        _ => false
    }
}

/**
 * Convert HTML string to AST.
 */
pub fn parse(template: &str, options: CompilerOptions) {

    let parser_options = ParserOptions {
        xml_mode: false,
        tokenizer_options: TokenizerOptions {
            xml_mode: Some(false),
            decode_entities: Some(true),
        }
    };

    let parser = Parser::new(template, &parser_options);

    for token in parser {

        if options.dev {
            if let Some(attrs) = token.attrs {
                for (attr_key, attr_value) in attrs {
                    if invalidAttributeRE.find(&attr_key).is_some() {
                        warn(
                            "Invalid dynamic argument expression: attribute names cannot contain spaces, quotes, <, >, / or =."
                        )
                    }
                }
            }
        }
    }
}
