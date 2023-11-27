mod util;
mod ast_element;
mod helpers;
mod uni_codes;

#[macro_use]
extern crate lazy_static;

use std::borrow::Cow;
use lazy_static::lazy_static;
use regex::Regex;
use rs_html_parser::{Parser, ParserOptions};
use rs_html_parser_tokenizer::TokenizerOptions;
use rs_html_parser_tokens::Token;
use rs_html_parser_tokens::TokenKind::OpenTag;

use crate::ast_element::{ASTElement, create_ast_element};
use crate::helpers::{get_and_remove_attr, get_and_remove_attr_impl};
use crate::uni_codes::{UC_TYPE, UC_V_PRE, UC_V_FOR};
use crate::util::{get_attribute, has_attribute, is_pre_tag_default};

lazy_static! {
    static ref INVALID_ATTRIBUTE_RE: Regex = Regex::new(r##"/[\s"'<>\/=]/"##).unwrap();
    static ref FOR_ALIAS_RE: Regex = Regex::new(r"([\s\S]*?)\s+(?:in|of)\s+([\s\S]*)").unwrap();
    static ref FOR_ITERATOR_RE: Regex = Regex::new(r",([^,\}\]]*)(?:,([^,\}\]]*))?$").unwrap();
    static ref STRIP_PARENS_RE: Regex = Regex::new(r"^\(|\)$").unwrap();
    static ref DYNAMIC_ARG_RE: Regex = Regex::new(r"^\[.*\]$").unwrap();
    static ref ARG_RE: Regex = Regex::new(r":(.*)$").unwrap();
    static ref BIND_RE: Regex = Regex::new(r"^:|^\.|^v-bind:").unwrap();
    static ref PROP_BIND_RE: Regex = Regex::new(r"^\.").unwrap();
    static ref MODIFIER_RE: Regex = Regex::new(r"\.[^.\]]+(?=[^\]]*$)").unwrap();
    static ref SLOT_RE: Regex = Regex::new(r"^v-slot(:|$)|^#").unwrap();
    static ref LINE_BREAK_RE: Regex = Regex::new(r"[\r\n]").unwrap();
    static ref WHITESPACE_RE: Regex = Regex::new(r"[ \f\t\r\n]+").unwrap();
}


// TODO: Move to options
fn warn(message: &str) {
    println!("{}", message)
}

struct CompilerOptions {
    dev: bool,
    is_ssr: bool,

    is_pre_tag: Option<fn(tag: &str) -> bool>
}


fn is_forbidden_tag(el: &Token) -> bool {
    if &el.kind != &OpenTag {
        return false;
    }

    match &el.data {
        Cow::Borrowed("style") => true,
        Cow::Borrowed("script") => {
            let attr_value = get_attribute(el, &UC_TYPE);

            if let Some((val, _quote)) = attr_value {
                return val == "text/javascript";
            }

            return false;
        }
        _ => false
    }
}

fn process_pre(mut el: ASTElement) -> ASTElement {
    if get_and_remove_attr_impl(&mut el.token.attrs, &mut el.ignored, &UC_V_PRE, false).is_some() {
        el.pre = true;
    }

    el
}

/**
 * Convert HTML string to AST.
 */
pub fn parse(template: &str, options: CompilerOptions) {

    let platform_is_pre_tag: fn(tag: &str) -> bool = match options.is_pre_tag {
        None => |x| false,
        Some(v) => v
    };

    // let mut root;
    // let mut current_parent;
    let mut in_v_pre: bool = false;
    let mut in_pre: bool = false;
    let mut warned: bool = false;

    let warned_once = |msg: &str| {
        if !warned {
            warned = true;
            warn(msg)
        }
    };

    let parser_options = ParserOptions {
        xml_mode: false,
        tokenizer_options: TokenizerOptions {
            xml_mode: Some(false),
            decode_entities: Some(true),
        },
    };

    let parser = Parser::new(template, &parser_options);

    for token in parser {
        let mut element = create_ast_element(token);

        if options.dev {
            if let Some(attrs) = &element.token.attrs {
                for (attr_key, attr_value) in attrs {
                    if INVALID_ATTRIBUTE_RE.find(&attr_key).is_some() {
                        warn(
                            "Invalid dynamic argument expression: attribute names cannot contain spaces, quotes, <, >, / or =."
                        )
                    }
                }
            }
        }

        if is_forbidden_tag(&element.token) && !options.is_ssr {
            element.forbidden = true;

            if options.dev {
                // TODO: add tag
                warn("
            Templates should only be responsible for mapping the state to the
            UI. Avoid placing tags with side-effects in your templates, such as
            <{tag}> as they will not be parsed.
                ")
            }
        }

        // TODO Apply pre-transforms

        if !in_v_pre {
            element = process_pre(element);
            if element.pre {
                in_v_pre = true;
            }
        }
        if platform_is_pre_tag(&element.token.data) {
            in_pre = true;
        }
        if in_v_pre {
            process_raw_attributes(&mut element)
        } else if !element.processed {
            element = process_for(element);
        }
    }
}

fn process_raw_attributes(el: &mut ASTElement) {
    // processing attributes should not be needed
    if el.token.attrs.is_none() {
        // non root node in pre blocks with no attributes
        el.plain = true;
    }
}

fn process_for(mut el: ASTElement) -> ASTElement {
    let val = get_and_remove_attr_impl(&mut el.token.attrs, &mut el.ignored, &UC_V_FOR, false);
    if let Some(v_for_val) = val {
        let result = parse_for(&v_for_val);
    }
    el
}

pub struct ForParseResult {
    pub alias: String,
    pub for_value: String,
    pub iterator1: Option<String>,
    pub iterator2: Option<String>,
}

pub fn parse_for(exp: &str) -> Option<ForParseResult> {
    if let Some(in_match) = FOR_ALIAS_RE.captures(exp) {
        let mut res = ForParseResult {
            alias: STRIP_PARENS_RE.replace_all(in_match[1].trim(), "").to_string(),
            for_value: in_match[2].trim().to_string(),
            iterator1: None,
            iterator2: None,
        };

        let alias = res.alias.clone();
        if let Some(iterator_match) = FOR_ITERATOR_RE.captures(&alias) {
            res.alias = iterator_match[1].trim().to_string();
            res.iterator1 = Some(iterator_match[1].trim().to_string());
            if let Some(iterator2) = iterator_match.get(2) {
                res.iterator2 = Some(iterator2.as_str().trim().to_string());
            }
        }

        Some(res)
    } else {
        None
    }
}

