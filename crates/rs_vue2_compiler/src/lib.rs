mod util;
mod ast_elements;
mod helpers;
mod uni_codes;
mod ast_tree;
mod filter_parser;

#[macro_use]
extern crate lazy_static;

use std::collections::VecDeque;
use lazy_static::lazy_static;
use regex::Regex;
use rs_html_parser::{Parser, ParserOptions};
use rs_html_parser_tokenizer::TokenizerOptions;
use rs_html_parser_tokens::{Token, TokenKind};
use crate::ast_elements::{ASTElement, create_ast_element};
use crate::ast_tree::ASTTree;
use crate::uni_codes::{UC_TYPE, UC_V_FOR};
use crate::util::{get_attribute, has_attribute};

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
    if &el.kind != &TokenKind::OpenTag {
        return false;
    }

    match &*el.data {
        "style" => true,
        "script" => {
            let attr_value = get_attribute(el, &UC_TYPE);

            if let Some((val, _quote)) = attr_value {
                return &**val == "text/javascript";
            }

            return false;
        }
        _ => false
    }
}

pub struct VueParser<'a> {
    options: CompilerOptions,

    stack: VecDeque<ASTElement>,
    current_parent: Option<&'a ASTElement>,

    in_v_pre: bool,
    in_pre: bool,
    warned: bool,
}

const PARSER_OPTIONS: ParserOptions = ParserOptions {
    xml_mode: false,
    tokenizer_options: TokenizerOptions {
    xml_mode: Some(false),
    decode_entities: Some(true),
    },
};

impl<'i> VueParser<'i> {
    pub fn new(options: CompilerOptions) -> VueParser<'i> {
        VueParser {
            options,
            stack: Default::default(),
            current_parent: None,
            in_v_pre: false,
            in_pre: false,
            warned: false,
        }
    }

    fn warn_once(&mut self, msg: &str) {
        if !self.warned {
            self.warned = true;
            warn(msg);
        }
    }

    fn check_root_constraints(&mut self, new_root: &ASTElement ) {
        if self.warned {
           return;
        }

        if new_root.token.data.eq_ignore_ascii_case("slot")
            || new_root.token.data.eq_ignore_ascii_case("template") {
            self.warn_once("Cannot use <${el.tag}> as component root element because it may contain multiple nodes.")
        }
        if has_attribute(&new_root.token, &UC_V_FOR) {
            self.warn_once("Cannot use v-for on stateful component root element because it renders multiple elements.")
        }
    }

    fn platform_is_pre_tag(&mut self, tag: &str) -> bool {
        if let Some(pre_tag_fn) = self.options.is_pre_tag {
            return pre_tag_fn(tag);
        }

        return false;
    }

    pub fn parse(&'i mut self, template: &'i str) -> Option<ASTTree<'i>> {
        let parser = Parser::new(template, &PARSER_OPTIONS);
        let is_dev = self.options.dev;
        let mut tree: Option<ASTTree> = None;
        let mut stack: VecDeque<ASTElement> = Default::default();
        let mut current_parent: Option<&ASTElement>;

        for token in parser {
            match token.kind {
                TokenKind::OpenTag => {
                    let mut element = create_ast_element(token);

                    if is_dev {
                        if let Some(attrs) = &element.token.attrs {
                            for (attr_key, _attr_value) in attrs {
                                if INVALID_ATTRIBUTE_RE.find(&attr_key).is_some() {
                                    warn(
                                        "Invalid dynamic argument expression: attribute names cannot contain spaces, quotes, <, >, / or =."
                                    )
                                }
                            }
                        }
                    }

                    if is_forbidden_tag(&element.token) && !self.options.is_ssr {
                        element.forbidden = true;

                        if is_dev {
                            // TODO: add tag
                            warn("
            Templates should only be responsible for mapping the state to the
            UI. Avoid placing tags with side-effects in your templates, such as
            <{tag}> as they will not be parsed.
                ")
                        }
                    }

                    // TODO Apply pre-transforms

                    if !self.in_v_pre {
                        element.process_pre();
                        if element.pre {
                            self.in_v_pre = true;
                        }
                    }
                    if self.platform_is_pre_tag(&element.token.data) {
                        self.in_pre = true;
                    }
                    if self.in_v_pre {
                        element.process_raw_attributes()
                    } else if !element.processed {
                        element.process_for();
                        element.process_if();
                        element.process_once();
                    }

                    current_parent = Some(&element);

                    match tree {
                        None => {
                            if is_dev {
                                self.check_root_constraints(&element);
                            }
                            tree = Some(ASTTree::new(element));
                        }
                        Some(_) => {
                            stack.push_back(element);
                        }
                    }
                },
                TokenKind::CloseTag => {
                    let el_option = self.stack.pop_back();

                    if let Some(mut el) = el_option {
                        // trim white space ??

                        if !self.in_v_pre && !el.processed {
                            el.process_element();
                        }
                    }
                },
                TokenKind::Text => {

                }
                _ => {
                    todo!("missing implementation")
                }
            }
        }

        tree
    }
}
