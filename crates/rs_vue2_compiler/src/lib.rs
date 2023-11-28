mod util;
mod ast_elements;
mod helpers;
mod uni_codes;
mod ast_tree;

#[macro_use]
extern crate lazy_static;

use std::borrow::Cow;
use std::collections::VecDeque;
use std::rc::Rc;
use lazy_static::lazy_static;
use regex::Regex;
use rs_html_parser::{Parser, ParserOptions};
use rs_html_parser_tokenizer::TokenizerOptions;
use rs_html_parser_tokens::{Token, TokenKind};

use crate::ast_elements::{ASTElement, create_ast_element};
use crate::ast_tree::{ASTNode, ASTTree};
use crate::helpers::{get_and_remove_attr};
use crate::uni_codes::{UC_TYPE, UC_V_PRE, UC_V_FOR, UC_V_IF, UC_V_ELSE, UC_V_ELSE_IF, UC_V_ONCE};
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
    if get_and_remove_attr(&mut el.token.attrs, &mut el.ignored, &UC_V_PRE, false).is_some() {
        el.pre = true;
    }

    el
}

pub struct VueParser<'a> {
    options: CompilerOptions,

    stack: VecDeque<ASTElement<'a>>,
    root: Option<ASTElement<'a>>,
    current_parent: Option<&'a ASTElement<'a>>,

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
            root: None,
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

    // fn check_root_constraints(&mut self) {
    //     if self.warned {
    //        return;
    //     }
    //     let el = self.root.unwrap();
    //
    //     if el.token.data.eq_ignore_ascii_case("slot")
    //         || el.token.data.eq_ignore_ascii_case("template") {
    //         self.warn_once("Cannot use <${el.tag}> as component root element because it may contain multiple nodes.")
    //     }
    //     if has_attribute(&el.token, &UC_V_FOR) {
    //         self.warn_once("Cannot use v-for on stateful component root element because it renders multiple elements.")
    //     }
    // }

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
                        element = process_pre(element);
                        if element.pre {
                            self.in_v_pre = true;
                        }
                    }
                    if self.platform_is_pre_tag(&element.token.data) {
                        self.in_pre = true;
                    }
                    if self.in_v_pre {
                        process_raw_attributes(&mut element)
                    } else if !element.processed {
                        element = process_for(element);
                        element = process_if(element);
                        element = process_once(element);
                    }

                    current_parent = Some(&element);

                    match tree {
                        None => {
                            tree = Some(ASTTree::new(element));
                            if is_dev {
                                // self.check_root_constraints()
                            }
                        }
                        Some(_) => {
                            stack.push_back(element);
                        }
                    }
                },
                TokenKind::CloseTag => {
                    let el_option = self.stack.pop_back();

                    if let Some(el) = el_option {
                        process_element(el);
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

fn process_raw_attributes(el: &mut ASTElement) {
    // processing attributes should not be needed
    if el.token.attrs.is_none() {
        // non root node in pre blocks with no attributes
        el.plain = true;
    }
}

fn process_for(mut el: ASTElement) -> ASTElement {
    let val = get_and_remove_attr(&mut el.token.attrs, &mut el.ignored, &UC_V_FOR, false);
    if let Some(v_for_val) = val {
        let result_option = parse_for(&v_for_val);

        if let Some(result) = result_option {
            el.alias = Some(result.alias);
            el.for_value = Some(result.for_value);
            el.iterator1 = result.iterator1;
            el.iterator2 = result.iterator2;
        } else {
            // TODO
            warn("Invalid v-for expression: ${exp}")
        }
    }

    el
}

struct ForParseResult {
    pub alias: String,
    pub for_value: String,
    pub iterator1: Option<String>,
    pub iterator2: Option<String>,
}

fn parse_for(exp: &str) -> Option<ForParseResult> {
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

fn process_if(mut el: ASTElement) -> ASTElement {
    let vif_optional = get_and_remove_attr(
        &mut el.token.attrs,
        &mut el.ignored,
        &UC_V_IF,
        false,
    );

    if let Some(vif_value) = vif_optional {
        el.if_val = Some(vif_value);
    } else {
        let v_else_optional = get_and_remove_attr(
            &mut el.token.attrs,
            &mut el.ignored,
            &UC_V_ELSE,
            false,
        );

        if v_else_optional.is_some() {
            el.is_else = true
        }

        let v_else_if_optional = get_and_remove_attr(
            &mut el.token.attrs,
            &mut el.ignored,
            &UC_V_ELSE_IF,
            false,
        );

        if let Some(v_else_if_val) = v_else_if_optional {
            el.if_val = Some(v_else_if_val);
        }
    }

    el
}

fn process_once(mut el: ASTElement) -> ASTElement {
    let v_once_optional = get_and_remove_attr(
        &mut el.token.attrs,
        &mut el.ignored,
        &UC_V_ONCE,
        false,
    );

    if v_once_optional.is_some() {
        el.once = true
    }

    el
}

fn process_element(mut el: ASTElement) -> ASTElement {
    el
}
