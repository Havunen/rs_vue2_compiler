use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::mem::take;
use rs_html_parser_tokenizer_tokens::QuoteType;
use rs_html_parser_tokens::{Token, TokenKind};
use unicase::UniCase;

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
}


pub fn create_ast_element(token: Token) -> ASTElement {
    ASTElement {
        token,
        forbidden: false,
        pre: false,
        plain: false,
        ignored: Default::default(),
        processed: false,
    }
}
