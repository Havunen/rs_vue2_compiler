use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::{Rc, Weak};
use rs_html_parser_tokens::Token;
use unicase::UniCase;

#[derive(Debug)]
pub struct ASTIfCondition<'a> {
    pub exp: &'a Option<&'a str>,
    pub block: &'a ASTElement<'a>,
}

#[derive(Debug)]
pub struct ASTElement<'a> {
    // rs_html_parser_tokens Token
    pub token: Token<'a>,
        
    // extra
    pub forbidden: bool,
    pub pre: bool,
    pub plain: bool,
    pub ignored: BTreeSet<UniCase<&'a str>>,
    pub processed: bool,

    // for
    pub alias: Option<String>,
    pub for_value: Option<String>,
    pub iterator1: Option<String>,
    pub iterator2: Option<String>,

    // if
    pub if_val: Option<Cow<'a, str>>,
    pub if_processed: bool,
    pub else_if_val: Option<Cow<'a, str>>,
    pub is_else: bool,

    pub once: bool,
}


pub fn create_ast_element(token: Token) -> ASTElement {
    ASTElement {
        token,
        forbidden: false,
        pre: false,
        plain: false,
        ignored: Default::default(),
        processed: false,
        alias: None,
        for_value: None,
        iterator1: None,
        iterator2: None,
        if_val: None,
        if_processed: false,
        else_if_val: None,
        is_else: false,
        once: false,
    }
}
