pub mod ast_tree;
mod directives_model;
mod filter_parser;
mod helpers;
mod text_parser;
mod uni_codes;
mod util;
mod warn_logger;
pub mod web;

extern crate lazy_static;

use crate::ast_tree::{
    create_ast_element, ASTElement, ASTElementKind, ASTNode, ASTTree, IfCondition,
};
use crate::text_parser::parse_text;
use crate::uni_codes::{UC_TYPE, UC_V_FOR};
use crate::util::{get_attribute_value, has_attribute};
use crate::warn_logger::WarnLogger;
use crate::web::element::get_namespace;
use lazy_static::lazy_static;
use regex::Regex;
use rs_html_parser::{Parser, ParserOptions};
use rs_html_parser_tokenizer::TokenizerOptions;
use rs_html_parser_tokens::{Token, TokenKind};
use std::cell::{RefCell, RefMut};
use std::collections::VecDeque;
use std::format;
use std::rc::Rc;
use unicase_collections::unicase_btree_map::UniCaseBTreeMap;

lazy_static! {
    static ref INVALID_ATTRIBUTE_RE: Regex = Regex::new(r##"/[\s"'<>\/=]/"##).unwrap();
    static ref FOR_ALIAS_RE: Regex = Regex::new(r"([\s\S]*?)\s+(?:in|of)\s+([\s\S]*)").unwrap();
    static ref FOR_ITERATOR_RE: Regex = Regex::new(r",([^,\}\]]*)(?:,([^,\}\]]*))?$").unwrap();
    static ref STRIP_PARENS_RE: Regex = Regex::new(r"^\(|\)$").unwrap();
    static ref DYNAMIC_ARG_RE: Regex = Regex::new(r"^\[.*\]$").unwrap();
    static ref ARG_RE: Regex = Regex::new(r":(.*)$").unwrap();
    static ref BIND_RE: Regex = Regex::new(r"^:|^\.|^v-bind:").unwrap();
    static ref PROP_BIND_RE: Regex = Regex::new(r"^\.").unwrap();
    static ref MODIFIER_RE: Regex = Regex::new(r"\.[^.\]]+").unwrap(); // This regex has been refactored not to include look-ahead
    static ref SLOT_RE: Regex = Regex::new(r"^v-slot(:|$)|^#").unwrap();
    static ref LINE_BREAK_RE: Regex = Regex::new(r"[\r\n]").unwrap();
    static ref WHITESPACE_RE: Regex = Regex::new(r"[ \f\t\r\n]+").unwrap();
    static ref DIR_RE: Regex = Regex::new(r"^(v-|@|:|#)").unwrap();
    static ref DIR_RE_VBIND_SHORT_HAND: Regex = Regex::new(r"^v-|^@|^:|^\.|^#").unwrap();
    static ref ON_RE: Regex = Regex::new(r"^@|^v-on:").unwrap();
}

// TODO: Move to options
fn print_line(message: &str) {
    println!("{}", message)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WhitespaceHandling {
    Condense,
    Preserve,
    Ignore,
}

pub struct CompilerOptions {
    pub dev: bool,
    pub is_ssr: bool,

    pub v_bind_prop_short_hand: bool,
    pub preserve_comments: bool,
    pub whitespace_handling: WhitespaceHandling,
    pub new_slot_syntax: bool,

    pub is_pre_tag: Option<fn(tag: &str) -> bool>,
    pub get_namespace: Option<fn(tag: &str) -> Option<&'static str>>,
    pub warn: Option<Box<dyn WarnLogger>>,
    pub delimiters: Option<(String, String)>,

    pub modules: Option<Vec<Box<dyn ModuleApi>>>,
}

pub trait ModuleApi {
    fn transform_node(&self, node: &mut ASTNode, options: &CompilerOptions);
    fn gen_data(&self, node: &ASTNode) -> Option<String>;
    fn static_keys(&self) -> Vec<&'static str>;
    fn pre_transform_node(
        &self,
        node: &mut ASTNode,
        tree: &mut ASTTree,
        options: &CompilerOptions,
    ) -> Option<Rc<RefCell<ASTNode>>>;
}

fn is_forbidden_tag(el: &Token) -> bool {
    if &el.kind != &TokenKind::OpenTag {
        return false;
    }

    match &*el.data {
        "style" => true,
        "script" => {
            let attr_entry = get_attribute_value(el, &UC_TYPE);

            if let Some(some_attr_entry) = attr_entry {
                if let Some((val, _quotes)) = some_attr_entry {
                    return val.as_ref() == "text/javascript";
                }
            }

            return false;
        }
        _ => false,
    }
}

pub struct VueParser<'a> {
    dev: bool,
    warn: Box<dyn WarnLogger>,

    is_ssr: bool,
    is_pre_tag: fn(tag: &str) -> bool,

    preserve_comments: bool,
    whitespace_handling: WhitespaceHandling,

    in_v_pre: bool,
    in_pre: bool,
    warned: bool,

    get_namespace: fn(tag: &str) -> Option<&'static str>,

    active_text: Option<String>,
    options: &'a CompilerOptions,
}

const PARSER_OPTIONS: ParserOptions = ParserOptions {
    xml_mode: false,
    tokenizer_options: TokenizerOptions {
        xml_mode: Some(false),
        decode_entities: Some(true),
    },
};

impl<'a> VueParser<'a> {
    pub fn new(options: &'a CompilerOptions) -> VueParser<'a> {
        VueParser {
            options: &options,
            dev: options.dev,
            is_pre_tag: options.is_pre_tag.unwrap_or(|_| false),
            is_ssr: options.is_ssr,
            in_v_pre: false,
            in_pre: false,
            warned: false,
            get_namespace: options.get_namespace.unwrap_or(get_namespace),
            whitespace_handling: options.whitespace_handling,
            preserve_comments: false,
            active_text: None,
            warn: options.warn.clone().unwrap_or_else(|| Box::new(print_line)),
        }
    }

    fn warn_once(&mut self, msg: &str) {
        if !self.warned {
            self.warned = true;
            self.warn.call(msg);
        }
    }

    fn check_root_constraints(&mut self, new_root: &ASTElement) {
        if self.warned {
            return;
        }

        if new_root.token.data.eq_ignore_ascii_case("slot")
            || new_root.token.data.eq_ignore_ascii_case("template")
        {
            self.warn_once(&format!(
                "Cannot use <{}> as component root element because it may contain multiple nodes.",
                new_root.token.data
            ))
        }
        if has_attribute(&new_root.token, &UC_V_FOR) {
            self.warn_once("Cannot use v-for on stateful component root element because it renders multiple elements.")
        }
    }

    pub fn parse(&mut self, template: &str) -> ASTTree {
        let parser = Parser::new(template, &PARSER_OPTIONS);
        let is_dev = self.dev;

        let mut root_tree: ASTTree = ASTTree::new(is_dev, self.warn.clone_box());
        let mut stack: VecDeque<usize> = VecDeque::new();
        let mut current_parent_id = 0;
        let mut current_namespace: Option<&'static str> = None;

        for token in parser {
            match token.kind {
                TokenKind::OpenTag => {
                    self.end_text_node(&mut root_tree, current_parent_id);

                    let mut node_rc = root_tree.create(
                        create_ast_element(token, ASTElementKind::Element),
                        current_parent_id,
                        is_dev,
                        self.warn.clone_box(),
                    );
                    let mut node = node_rc.borrow_mut();
                    let node_id = node.id;
                    root_tree.set(node_id, node_rc.clone());

                    let ns = if let Some(parent_ns) = current_namespace {
                        Some(parent_ns)
                    } else {
                        (self.get_namespace)(&node.el.token.data)
                    };

                    if let Some(namespace) = ns {
                        node.el.ns = Some(namespace);
                        current_namespace = Some(namespace);
                    }

                    if is_dev {
                        if let Some(attrs) = &node.el.token.attrs {
                            for (attr_key, _attr_value) in attrs {
                                if INVALID_ATTRIBUTE_RE.find(&attr_key).is_some() {
                                    self.warn.call(
                                        "Invalid dynamic argument expression: attribute names cannot contain spaces, quotes, <, >, / or =."
                                    )
                                }
                            }
                        }
                    }

                    if is_forbidden_tag(&node.el.token) && !self.is_ssr {
                        node.el.forbidden = true;

                        if is_dev {
                            // TODO: add tag
                            self.warn.call(
                                "
            Templates should only be responsible for mapping the state to the
            UI. Avoid placing tags with side-effects in your templates, such as
            <{tag}> as they will not be parsed.
                ",
                            )
                        }
                    }

                    if let Some(registered_modules) = &self.options.modules {
                        for module in registered_modules {
                            let possibly_new_node =
                                module.pre_transform_node(&mut node, &mut root_tree, self.options);

                            if let Some(new_node) = possibly_new_node {
                                drop(node);
                                node_rc = new_node;
                                node = node_rc.borrow_mut();
                            }
                        }
                    }

                    if !self.in_v_pre {
                        node.process_pre();
                        if node.el.pre {
                            self.in_v_pre = true;
                        }
                    }
                    if (self.is_pre_tag)(&node.el.token.data) {
                        self.in_pre = true;
                    }
                    if self.in_v_pre {
                        node.process_raw_attributes()
                    } else if !node.el.processed {
                        node.process_for();
                        node.process_if();
                        node.process_once();
                    }

                    current_parent_id = node_id;
                    stack.push_back(node_id);
                }
                TokenKind::CloseTag => {
                    self.end_text_node(&mut root_tree, current_parent_id);

                    let current_open_tag_id = stack.pop_back();
                    current_parent_id = *stack.back().unwrap_or(&(0usize));

                    if let Some(open_tag_id) = current_open_tag_id {
                        let node_ptr = root_tree.get(open_tag_id).unwrap();
                        let mut node = node_ptr.borrow_mut();
                        // trim white space ??

                        if !self.in_v_pre && !node.el.processed {
                            node.process_element(&root_tree, &self.options);
                        }
                        // tree management
                        if stack.is_empty() && node.id != 1 {
                            if root_tree.get(1).unwrap().borrow().el.if_val.is_some()
                                && (node.el.else_if_val.is_some() || node.el.is_else)
                            {
                                if is_dev {
                                    self.check_root_constraints(&node.el);
                                }
                                let else_if_val = node.el.else_if_val.clone();
                                let node_id = node.id;

                                node.add_if_condition(IfCondition {
                                    exp: else_if_val,
                                    block_id: node_id,
                                });
                            } else if is_dev {
                                self.warn.call("Component template should contain exactly one root element. If you are using v-if on multiple elements, use v-else-if to chain them instead.");
                            }
                        }
                        let mut current_parent =
                            root_tree.get(node.parent_id).unwrap().borrow_mut();

                        // always take root node, even if forbidden
                        if !node.el.forbidden || node.id == 1 {
                            if is_dev && node.id == 1 {
                                self.check_root_constraints(&node.el);
                            }
                            if node.el.else_if_val.is_some() || node.el.is_else {
                                node.process_if_conditions(
                                    node_ptr,
                                    current_parent.children.as_mut(),
                                );
                            } else {
                                if node.el.slot_scope.is_some() {
                                    // scoped slot
                                    // keep it in the children list so that v-else(-if) conditions can
                                    // find it as the prev node.
                                    let scoped_slots = current_parent
                                        .el
                                        .scoped_slots
                                        .get_or_insert(UniCaseBTreeMap::new());

                                    let slot_target = node.el.slot_target.clone();
                                    let name = if let Some(slot_target) = slot_target {
                                        slot_target
                                    } else {
                                        "\"default\"".to_string()
                                    };

                                    scoped_slots.insert(name, node_ptr.clone());
                                }

                                let children: &mut Vec<Rc<RefCell<ASTNode>>> =
                                    current_parent.children.as_mut();
                                children.push(node_ptr.clone());

                                // TODO: This is most likely unnecessary, verify later
                                if current_parent.id != current_parent_id {
                                    println!("parent id does not match current parent id");
                                }
                            }
                        }

                        // final children cleanup
                        // filter out scoped slots
                        node.children = node
                            .children
                            .iter()
                            .map(|child| Rc::clone(child))
                            .filter_map(|child_rc| {
                                let child = child_rc.borrow_mut();
                                if child.el.slot_scope.is_none() {
                                    Some(Rc::clone(&child_rc))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();

                        // remove trailing whitespace node again

                        if node.el.pre {
                            self.in_v_pre = false
                        }
                        if (self.is_pre_tag)(&node.el.token.data) {
                            self.in_pre = false
                        }

                        // apply post-transforms
                        // for (let i = 0; i < postTransforms.length; i++) {
                        //     postTransforms[i](element, options)
                        // }
                    }
                }
                TokenKind::Comment => {
                    if !self.preserve_comments {
                        continue;
                    }
                    if current_parent_id != 0 {
                        self.end_text_node(&mut root_tree, current_parent_id);

                        let node_rc = root_tree.create(
                            create_ast_element(token, ASTElementKind::Text),
                            current_parent_id,
                            is_dev,
                            self.warn.clone_box(),
                        );
                        let mut node = node_rc.borrow_mut();
                        node.el.is_comment = true;
                        root_tree.set(node.id, node_rc.clone());
                        current_parent_id = node.id;
                        stack.push_back(node.id);
                    }
                }
                TokenKind::CommentEnd => {
                    if !self.preserve_comments {
                        continue;
                    }

                    if current_parent_id != 0 {
                        self.end_text_node(&mut root_tree, current_parent_id);

                        let _unused_open_comment_id = stack.pop_back();
                        current_parent_id = *stack.back().unwrap_or(&(0usize));
                    }
                }
                TokenKind::Text => {
                    if current_parent_id == 0 {
                        if is_dev {
                            if &token.data.as_ref() == &template {
                                self.warn.call("Component template requires a root element, rather than just text.")
                            } else {
                                let text_trimmed = token.data.trim();

                                if !text_trimmed.is_empty() {
                                    self.warn.call(&format!(
                                        "text \"{}\" outside root element will be ignored.",
                                        text_trimmed
                                    ));
                                }
                            }
                        }

                        continue;
                    }

                    let text = self.condense_whitespace(&root_tree, current_parent_id, &token.data);

                    if !text.is_empty() {
                        if let Some(active_text) = &mut self.active_text {
                            *active_text += &text;
                        } else {
                            self.active_text = Some(text);
                        }
                    }
                }
                _ => {}
            }
        }

        root_tree
    }

    fn end_text_node(&mut self, root_tree: &mut ASTTree, current_parent_id: usize) {
        if let Some(active_text) = &self.active_text {
            let parse_text_result: Option<(String, Vec<String>)>;
            let final_text = if self.whitespace_handling == WhitespaceHandling::Condense {
                WHITESPACE_RE.replace_all(active_text, " ").to_string()
            } else {
                active_text.to_string()
            };

            if !&final_text.is_empty() {
                if !self.in_v_pre {
                    parse_text_result = parse_text(&final_text, &None);
                } else {
                    parse_text_result = None;
                }

                let node_rc: Rc<RefCell<ASTNode>>;
                let mut node: RefMut<ASTNode>;
                if let Some(expression_text) = parse_text_result {
                    node_rc = root_tree.create(
                        create_ast_element(
                            Token {
                                data: final_text.into_boxed_str(),
                                attrs: None,
                                kind: TokenKind::Text,
                                is_implied: false,
                            },
                            ASTElementKind::Expression,
                        ),
                        current_parent_id,
                        self.dev,
                        self.warn.clone_box(),
                    );
                    node = node_rc.borrow_mut();
                    node.el.expression = Some(expression_text.0);
                    node.el.tokens = Some(expression_text.1);
                } else {
                    node_rc = root_tree.create(
                        create_ast_element(
                            Token {
                                data: final_text.into_boxed_str(),
                                attrs: None,
                                kind: TokenKind::Text,
                                is_implied: false,
                            },
                            ASTElementKind::Text,
                        ),
                        current_parent_id,
                        self.dev,
                        self.warn.clone_box(),
                    );
                    node = node_rc.borrow_mut();
                }

                root_tree
                    .get(current_parent_id)
                    .unwrap()
                    .borrow_mut()
                    .children
                    .push(node_rc.clone());
                root_tree.set(node.id, node_rc.clone());
            }

            self.active_text = None;
        }
    }

    fn condense_whitespace(
        &mut self,
        root_tree: &ASTTree,
        current_parent_id: usize,
        text: &str,
    ) -> String {
        return if self.in_pre {
            text.to_string()
        } else if !text.trim().is_empty() {
            if self.whitespace_handling == WhitespaceHandling::Condense {
                WHITESPACE_RE.replace_all(text, " ").to_string()
            } else {
                text.to_string()
            }
        } else if root_tree
            .get(current_parent_id)
            .unwrap()
            .borrow()
            .children
            .is_empty()
        {
            // remove the whitespace-only node right after an opening tag
            String::new()
        } else if self.whitespace_handling == WhitespaceHandling::Condense {
            // in condense mode, remove the whitespace node if it contains
            // line break, otherwise condense to a single space
            if LINE_BREAK_RE.is_match(text) {
                String::new()
            } else {
                " ".to_string()
            }
        } else if self.whitespace_handling == WhitespaceHandling::Preserve {
            " ".to_string()
        } else {
            String::new()
        };
    }
}
