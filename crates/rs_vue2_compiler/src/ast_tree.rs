use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::collections::HashMap;
use rs_html_parser_tokenizer_tokens::QuoteType;
use rs_html_parser_tokens::Token;
use rs_html_parser_tokens::TokenKind::ProcessingInstruction;
use unicase_collections::unicase_btree_set::UniCaseBTreeSet;
use crate::uni_codes::{UC_KEY, UC_V_ELSE, UC_V_ELSE_IF, UC_V_FOR, UC_V_IF, UC_V_ONCE, UC_V_PRE};
use crate::{FOR_ALIAS_RE, FOR_ITERATOR_RE, STRIP_PARENS_RE, warn};
use crate::filter_parser::parse_filters;

#[derive(Debug)]
pub struct ASTElement {
    // rs_html_parser_tokens Token
    pub token: Token,

    // internal helpers
    pub is_dev: bool,

    // extra
    pub forbidden: bool,
    pub pre: bool,
    pub plain: bool,
    pub ignored: UniCaseBTreeSet,
    pub processed: bool,
    pub ref_val: Option<String>,
    pub ref_in_for: bool,

    pub key: Option<String>,

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

    pub once: bool,

    pub scoped_slots: Option<(Box<str>, QuoteType)>,
}


pub fn create_ast_element(token: Token, is_dev: bool) -> ASTElement {
    ASTElement {
        token,
        forbidden: false,
        pre: false,
        plain: false,
        ignored: Default::default(),
        processed: false,
        ref_val: None,
        alias: None,
        for_value: None,
        iterator1: None,
        iterator2: None,
        if_val: None,
        if_processed: false,
        else_if_val: None,
        is_else: false,
        once: false,
        key: None,

        is_dev,
        ref_in_for: false,
        scoped_slots: None,
    }
}

#[derive(Debug)]
pub struct ASTNode {
    pub id: usize,
    pub el: ASTElement,
    pub children: Vec<Rc<RefCell<ASTNode>>>,
    pub parent: Option<Weak<RefCell<ASTNode>>>,
}

#[derive(Debug)]
pub struct ASTTree {
    pub root: Rc<RefCell<ASTNode>>,
    counter: usize,
    nodes: HashMap<usize, Rc<RefCell<ASTNode>>>,
}

impl ASTTree {
    pub fn new(is_dev: bool) -> Self {
        let node = Rc::new(RefCell::new(ASTNode {
            id: 0,
            el: create_ast_element(Token {
                kind: ProcessingInstruction,
                data: "".into(),
                attrs: None,
                is_implied: false,
            }, is_dev),
            children: Default::default(),
            parent: None,
        }));

        let mut tree = ASTTree {
            counter: 0,
            root: Rc::clone(&node),
            nodes: Default::default(),
        };

        tree.nodes.insert(0, Rc::clone(&node));

        return tree;
    }

    pub fn create(&mut self, element: ASTElement, parent_id: usize) -> Rc<RefCell<ASTNode>> {
        self.counter += 1;
        let mut parent = self.get(parent_id).cloned().unwrap();

        let new_node = Rc::new(RefCell::new(ASTNode {
            id: self.counter,
            el: element,
            parent: Some(Rc::downgrade(&parent)),
            children: vec![]
        }));

        parent.borrow_mut().children.push(Rc::clone(&new_node));

        // let parent = self.nodes.get(&parent_id).cloned();
        // let node = Rc::new(RefCell::new(ASTNode {
        //     id: self.counter,
        //     el: element,
        //     children: Vec::new(),
        //     parent: parent.as_ref().map(|p| Rc::downgrade(p)),
        // }));
        //
        // self.counter += 1;
        // self.nodes.insert(self.counter, Rc::clone(&node));
        //
        // if let Some(parent) = parent {
        //     parent.children.push(Rc::clone(&node));
        // }
        //
        // Rc::clone(&node)

        new_node
    }

    pub fn get(&self, id: usize) -> Option<&Rc<RefCell<ASTNode>>> {
        self.nodes.get(&id)
    }
}


#[derive(Debug)]
struct ForParseResult {
    pub alias: String,
    pub for_value: String,
    pub iterator1: Option<String>,
    pub iterator2: Option<String>,
}

impl ASTNode {

    pub fn process_raw_attributes(&mut self) {
        // processing attributes should not be needed
        if self.el.token.attrs.is_none() {
            // non root node in pre blocks with no attributes
            self.el.plain = true;
        }
    }

    pub fn process_for(&mut self) {
        let val = self.get_and_remove_attr(&UC_V_FOR, false);
        if let Some(v_for_val) = val {
            let v_for_val = v_for_val.clone(); // Clone the value to remove the borrow
            let result_option = self.parse_for(&v_for_val);

            if let Some(result) = result_option {
                self.el.alias = Some(result.alias);
                self.el.for_value = Some(result.for_value);
                self.el.iterator1 = result.iterator1;
                self.el.iterator2 = result.iterator2;
            } else {
                // TODO
                warn("Invalid v-for expression: ${exp}")
            }
        }
    }

    pub fn process_pre(&mut self) {
        if self.get_and_remove_attr(&UC_V_PRE, false).is_some() {
            self.el.pre = true;
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
            self.el.if_val = Some(vif_value.to_string());
        } else {
            let v_else_optional = self.get_and_remove_attr(
                &UC_V_ELSE,
                false,
            );

            if v_else_optional.is_some() {
                self.el.is_else = true
            }

            let v_else_if_optional = self.get_and_remove_attr(
                &UC_V_ELSE_IF,
                false,
            );

            if let Some(v_else_if_val) = v_else_if_optional {
                self.el.if_val = Some(v_else_if_val.to_string());
            }
        }
    }

    pub fn process_once(&mut self) {
        let v_once_optional = self.get_and_remove_attr(
            &UC_V_ONCE,
            false,
        );

        if v_once_optional.is_some() {
            self.el.once = true
        }
    }

    pub fn get_raw_attr(
        &self,
        name: &str,
    ) -> Option<&Box<str>> {
        if let Some(ref attrs) = self.el.token.attrs {
            if let Some(attr_value) = attrs.get(name) {
                if let Some((attr_value, _attr_quote)) = attr_value {
                    return Some(attr_value);
                }
            }
        }

        return None;
    }

    pub fn get_and_remove_attr(
        &mut self,
        name: &str,
        fully_remove: bool
    ) -> Option<&Box<str>> {
        if let Some(ref mut attrs) = self.el.token.attrs {
            if let Some(attr_value) = attrs.get(name) {
                if !fully_remove {
                    self.el.ignored.insert(name);
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
        if let Some(ref mut attrs) = self.el.token.attrs {
            if let Some(attr_value_option) = attrs.get(name) {
                if !fully_remove {
                    self.el.ignored.insert(name);
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
        let mut dynamic_value = self.get_and_remove_attr_including_quotes(&(":".to_string() + name), false);

        if dynamic_value.is_none() {
            dynamic_value = self.get_and_remove_attr_including_quotes(&("v-bind:".to_string() + name), false);
        }
        if let Some(found_dynamic_value) = dynamic_value {
            return parse_filters(&found_dynamic_value)
        }
        if get_static {
            let static_value = self.get_and_remove_attr(&name, false);
            if let Some(found_static_value) = static_value {
                return found_static_value.to_string()
            }
        }

        return String::from("")
    }

    pub fn get_raw_binding_attr(
        &mut self,
        name: &'static str
    ) -> Option<&Box<str>>  {
        let mut val = self.get_raw_attr(&(":".to_string() + name));

        if val.is_some() {
            return val;
        }

        val = self.get_raw_attr(&("v-bind:".to_string() + name));

        if val.is_some() {
            return val;
        }

        return self.get_raw_attr(&name);
    }

    pub fn process_element(&mut self) {
        self.process_key();

        // determine whether this is a plain element after
        // removing structural attributes
        self.el.plain = self.el.key.is_none() && self.el.scoped_slots.is_none() && self.el.token.attrs.is_none();

        self.process_ref();
    }

    pub fn process_key(&mut self) {
        let exp = self.get_binding_attr(&UC_KEY, false);

        if !exp.is_empty() {
            if self.el.is_dev {
                if self.el.token.data.eq_ignore_ascii_case("template") {
                    // self.get_raw_binding_attr(&UC_KEY).unwrap_or("".into()).to_string().as_str())
                    warn("<template> cannot be keyed. Place the key on real elements instead. {}");
                }

                let has_iterator_1 = self.el.iterator1.is_some() && self.el.iterator1.as_ref().unwrap().eq(&exp);
                let has_iterator_2 = self.el.iterator2.is_some() && self.el.iterator2.as_ref().unwrap().eq(&exp);

                if self.el.for_value.is_some() {
                    if has_iterator_1 || has_iterator_2 {
                        {
                            if let Some(parent) = self.parent.as_ref().unwrap().upgrade() {
                                if parent.borrow().el.token.data.eq_ignore_ascii_case("transition-group") {
                                    // getRawBindingAttr(el, 'key'),
                                    warn(
                                        r#"Do not use v-for index as key on <transition-group> children,
                                    "this is the same as not using keys. "#
                                    );
                                }
                            }
                        }
                    }
                }

                self.el.key = Some(exp);
            }
        }
    }
    fn process_ref(&mut self) {
        let ref_option = self.get_and_remove_attr("ref", false);

        if let Some(ref_value) = ref_option {
            self.el.ref_val = Some(ref_value.to_string());
            self.el.ref_in_for = self.check_in_for();
        }
    }

    pub fn check_in_for(&self) -> bool {
        if self.el.for_value.is_some() {
            return true;
        }

        let mut current_node = self.parent.as_ref().and_then(|parent_weak| parent_weak.upgrade());

        while let Some(node) = current_node {
            if node.borrow().el.for_value.is_some() {
                return true;
            }
            current_node = node.borrow().parent.as_ref().and_then(|parent_weak| parent_weak.upgrade());
        }

        false
    }
}
