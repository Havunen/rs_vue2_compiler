use rs_html_parser_tokenizer_tokens::QuoteType;
use rs_html_parser_tokens::Token;
use unicase_collections::unicase_btree_set::UniCaseBTreeSet;
use crate::uni_codes::{UC_KEY, UC_V_ELSE, UC_V_ELSE_IF, UC_V_FOR, UC_V_IF, UC_V_ONCE, UC_V_PRE};
use crate::{FOR_ALIAS_RE, FOR_ITERATOR_RE, STRIP_PARENS_RE, warn};
use crate::filter_parser::parse_filters;

#[derive(Debug)]
pub struct ASTElement {
    // rs_html_parser_tokens Token
    pub token: Token,
        
    // extra
    pub forbidden: bool,
    pub pre: bool,
    pub plain: bool,
    pub ignored: UniCaseBTreeSet,
    pub processed: bool,

    // for
    pub alias: Option<String>,
    pub for_value: Option<String>,
    pub iterator1: Option<String>,
    pub iterator2: Option<String>,

    // if
    pub if_val: Option<String>,
    pub if_processed: bool,
    pub else_if_val: Option<String>,
    pub is_else: bool,

    pub once: bool
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
        once: false
    }
}

struct ForParseResult {
    pub alias: String,
    pub for_value: String,
    pub iterator1: Option<String>,
    pub iterator2: Option<String>,
}

impl ASTElement {

    pub fn process_raw_attributes(&mut self) {
        // processing attributes should not be needed
        if self.token.attrs.is_none() {
            // non root node in pre blocks with no attributes
            self.plain = true;
        }
    }

    pub fn process_for(&mut self) {
        let val = self.get_and_remove_attr(&UC_V_FOR, false);
        if let Some(v_for_val) = val {
            let v_for_val = v_for_val.clone(); // Clone the value to remove the borrow
            let result_option = self.parse_for(&v_for_val);

            if let Some(result) = result_option {
                self.alias = Some(result.alias);
                self.for_value = Some(result.for_value);
                self.iterator1 = result.iterator1;
                self.iterator2 = result.iterator2;
            } else {
                // TODO
                warn("Invalid v-for expression: ${exp}")
            }
        }
    }

    pub fn process_pre(&mut self) {
        if self.get_and_remove_attr(&UC_V_PRE, false).is_some() {
            self.pre = true;
        }
    }

    pub fn parse_for(&mut self, exp: &str) -> Option<ForParseResult> {
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

    pub fn process_if(&mut self)  {
        let vif_optional = self.get_and_remove_attr(
            &UC_V_IF,
            false,
        );

        if let Some(vif_value) = vif_optional {
            self.if_val = Some(vif_value.to_string());
        } else {
            let v_else_optional = self.get_and_remove_attr(
                &UC_V_ELSE,
                false,
            );

            if v_else_optional.is_some() {
                self.is_else = true
            }

            let v_else_if_optional = self.get_and_remove_attr(
                &UC_V_ELSE_IF,
                false,
            );

            if let Some(v_else_if_val) = v_else_if_optional {
                self.if_val = Some(v_else_if_val.to_string());
            }
        }
    }

    pub fn process_once(&mut self) {
        let v_once_optional = self.get_and_remove_attr(
            &UC_V_ONCE,
            false,
        );

        if v_once_optional.is_some() {
            self.once = true
        }
    }

    pub fn process_element(&mut self) {
        self.process_key();
    }

    pub fn process_key(&mut self) {
        let exp = self.get_binding_attr(&UC_KEY, false);
    }


    pub fn get_and_remove_attr(
        &mut self,
        name: &str,
        fully_remove: bool
    ) -> Option<&Box<str>> {
        if let Some(ref mut attrs) = self.token.attrs {
            if let Some(attr_value) = attrs.get(name) {
                if !fully_remove {
                    self.ignored.insert(name);
                }

                if let Some((attr_value, _attr_quote)) = attr_value {
                    return Some(attr_value);
                }
            }
        }

        return None;
    }

    pub fn get_and_remove_attr_including_quotes(
        &mut self,
        name: &str,
        fully_remove: bool
    ) -> &Option<(Box<str>, QuoteType)> {
        if let Some(ref mut attrs) = self.token.attrs {
            if let Some(attr_value_option) = attrs.get(name) {
                if !fully_remove {
                    self.ignored.insert(name.clone());
                }

                return attr_value_option;
            }
        }

        return &None;
    }

    pub fn get_binding_attr(
        &mut self,
        name: &'static str,
        get_static: bool
    ) -> String  {
        let temp_string = ":".to_string() + name;
        let mut dynamic_value = self.get_and_remove_attr_including_quotes(&temp_string, false);

        if let Some(found_dynamic_value) = dynamic_value {
            return parse_filters(&found_dynamic_value)
        }

        return String::from("")
    }
}
