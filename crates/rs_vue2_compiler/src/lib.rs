pub mod ast_tree;
mod directives_model;
mod filter_parser;
mod helpers;
mod text_parser;
mod uni_codes;
mod util;
mod web;

extern crate lazy_static;

use crate::ast_tree::{
    create_ast_element, ASTElement, ASTElementKind, ASTNode, ASTTree, IfCondition,
};
use crate::text_parser::parse_text;
use crate::uni_codes::{UC_TYPE, UC_V_FOR};
use crate::util::{get_attribute, has_attribute};
use crate::web::element::get_namespace;
use lazy_static::lazy_static;
use regex::Regex;
use rs_html_parser::{Parser, ParserOptions};
use rs_html_parser_tokenizer::TokenizerOptions;
use rs_html_parser_tokens::{Token, TokenKind};
use std::cell::{RefCell, RefMut};
use std::collections::VecDeque;
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
    static ref MODIFIER_RE: Regex = Regex::new(r"\.[^.\]]+(?=[^\]]*$)").unwrap();
    static ref SLOT_RE: Regex = Regex::new(r"^v-slot(:|$)|^#").unwrap();
    static ref LINE_BREAK_RE: Regex = Regex::new(r"[\r\n]").unwrap();
    static ref WHITESPACE_RE: Regex = Regex::new(r"[ \f\t\r\n]+").unwrap();
    static ref DIR_RE: Regex = Regex::new(r"^(v-|@|:|#)").unwrap();
    static ref ON_RE: Regex = Regex::new(r"^@|^v-on:").unwrap();
}

// TODO: Move to options
fn warn(message: &str) {
    println!("{}", message)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WhitespaceHandling {
    Condense,
    Preserve,
    Ignore,
}

#[derive(Debug)]
pub struct CompilerOptions {
    pub dev: bool,
    pub is_ssr: bool,

    pub preserve_comments: bool,
    pub whitespace_handling: WhitespaceHandling,

    pub is_pre_tag: Option<fn(tag: &str) -> bool>,

    pub get_namespace: Option<fn(tag: &str) -> Option<&'static str>>,
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
        _ => false,
    }
}

pub struct VueParser {
    dev: bool,
    is_ssr: bool,
    is_pre_tag: fn(tag: &str) -> bool,

    preserve_comments: bool,
    whitespace_handling: WhitespaceHandling,

    in_v_pre: bool,
    in_pre: bool,
    warned: bool,

    get_namespace: fn(tag: &str) -> Option<&'static str>,

    active_text: Option<String>,
}

const PARSER_OPTIONS: ParserOptions = ParserOptions {
    xml_mode: false,
    tokenizer_options: TokenizerOptions {
        xml_mode: Some(false),
        decode_entities: Some(true),
    },
};

impl VueParser {
    pub fn new(options: CompilerOptions) -> VueParser {
        VueParser {
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
        }
    }

    fn warn_once(&mut self, msg: &str) {
        if !self.warned {
            self.warned = true;
            warn(msg);
        }
    }

    fn check_root_constraints(&mut self, new_root: &ASTElement) {
        if self.warned {
            return;
        }

        if new_root.token.data.eq_ignore_ascii_case("slot")
            || new_root.token.data.eq_ignore_ascii_case("template")
        {
            self.warn_once("Cannot use <${el.tag}> as component root element because it may contain multiple nodes.")
        }
        if has_attribute(&new_root.token, &UC_V_FOR) {
            self.warn_once("Cannot use v-for on stateful component root element because it renders multiple elements.")
        }
    }

    pub fn parse(&mut self, template: &str) -> ASTTree {
        let parser = Parser::new(template, &PARSER_OPTIONS);
        let is_dev = self.dev;
        let mut root_tree: ASTTree = ASTTree::new(is_dev);
        let mut stack: VecDeque<usize> = VecDeque::new();
        let mut current_parent_id = 0;
        let mut current_namespace: Option<&'static str> = None;

        for token in parser {
            match token.kind {
                TokenKind::OpenTag => {
                    self.end_text_node(&mut root_tree, current_parent_id);

                    let node_rc = root_tree.create(
                        create_ast_element(token, ASTElementKind::Element, is_dev),
                        current_parent_id,
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
                                    warn(
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
                            warn(
                                "
            Templates should only be responsible for mapping the state to the
            UI. Avoid placing tags with side-effects in your templates, such as
            <{tag}> as they will not be parsed.
                ",
                            )
                        }
                    }

                    // TODO Apply pre-transforms

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
                        let mut node = root_tree.get(open_tag_id).unwrap().borrow_mut();
                        // trim white space ??

                        if !self.in_v_pre && !node.el.processed {
                            node.process_element(&root_tree);
                        }
                        // tree management
                        if stack.is_empty() && node.id != 0 {
                            if root_tree.get(0).unwrap().borrow().el.if_val.is_some()
                                && (node.el.else_if_val.is_some() || node.el.is_else)
                            {
                                if is_dev {
                                    self.check_root_constraints(&node.el);
                                }
                                // TODO: What should happen if there is v-else ??? eh?
                                let else_if_val = node.el.else_if_val.clone();
                                let node_id = node.id;

                                node.add_if_condition(IfCondition {
                                    exp: else_if_val,
                                    block_id: node_id,
                                });
                            } else if is_dev {
                                warn("Component template should contain exactly one root element. If you are using v-if on multiple elements, use v-else-if to chain them instead.");
                            }
                        }
                        let mut current_parent =
                            root_tree.get(node.parent_id).unwrap().borrow_mut();
                        if
                        /* current_parent exists always && */
                        !node.el.forbidden {
                            if node.el.else_if_val.is_some() || node.el.is_else {
                                node.process_if_conditions(current_parent.children.as_mut());
                            } else {
                                if node.el.slot_scope.is_some() {
                                    // scoped slot
                                    // keep it in the children list so that v-else(-if) conditions can
                                    // find it as the prev node.
                                    let slot_target = node.el.slot_target.clone();
                                    let id = node.id;
                                    let scoped_slots =
                                        node.el.scoped_slots.get_or_insert(UniCaseBTreeMap::new());
                                    let name = if let Some(slot_target) = slot_target {
                                        slot_target
                                    } else {
                                        "\"default\"".to_string()
                                    };

                                    scoped_slots.insert(name, root_tree.get(id).unwrap().clone());

                                    // TODO: This is most likely unnecessary, verify later
                                    if let Some(parent) = node.parent.as_ref().unwrap().upgrade() {
                                        if parent.borrow().id != current_parent_id {
                                            println!("parent id does not match current parent id");
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
                        }
                    }
                }
                TokenKind::Comment => {
                    if !self.preserve_comments {
                        continue;
                    }
                    if current_parent_id != 0 {
                        self.end_text_node(&mut root_tree, current_parent_id);

                        let node_rc = root_tree.create(
                            create_ast_element(token, ASTElementKind::Text, is_dev),
                            current_parent_id,
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
                            // TODO: Simplified error msg
                            warn("text outside root element will be ignored..");
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

            if !self.in_v_pre {
                parse_text_result = parse_text(&active_text, None);
            } else {
                parse_text_result = None;
            }

            let final_text = if self.whitespace_handling == WhitespaceHandling::Condense {
                WHITESPACE_RE.replace_all(active_text, " ").to_string()
            } else {
                active_text.to_string()
            };

            let node_rc: Rc<RefCell<ASTNode>>;
            let mut node: RefMut<ASTNode>;
            if let Some(expression_text) = parse_text_result {
                node_rc = root_tree.create(
                    create_ast_element(Token {
                        data: final_text.into_boxed_str(),
                        attrs: None,
                        kind: TokenKind::Text,
                        is_implied: false,
                    }, ASTElementKind::Expression, self.dev),
                    current_parent_id,
                );
                node = node_rc.borrow_mut();
                node.el.expression = Some(expression_text.0);
                node.el.tokens = Some(expression_text.1);
            } else {
                node_rc = root_tree.create(
                    create_ast_element(Token {
                        data: final_text.into_boxed_str(),
                        attrs: None,
                        kind: TokenKind::Text,
                        is_implied: false,
                    }, ASTElementKind::Text, self.dev),
                    current_parent_id,
                );
                node = node_rc.borrow_mut();
            }

            root_tree.set(node.id, node_rc.clone());

            self.active_text = None;
        }
    }

    fn condense_whitespace(&mut self, root_tree: &ASTTree, current_parent_id: usize, text: &str) -> String {
        return if self.in_pre {
            text.to_string()
        } else if !text.trim().is_empty() {
            if self.whitespace_handling == WhitespaceHandling::Condense {
                WHITESPACE_RE.replace_all(text, " ").to_string()
            } else {
                text.to_string()
            }
        }
        else if root_tree.get(current_parent_id).unwrap().borrow().children.is_empty() {
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
