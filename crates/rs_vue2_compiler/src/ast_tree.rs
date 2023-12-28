use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::collections::HashMap;
use rs_html_parser_tokenizer_tokens::QuoteType;
use rs_html_parser_tokens::Token;
use rs_html_parser_tokens::TokenKind::{OpenTag, ProcessingInstruction};
use unicase_collections::unicase_btree_map::UniCaseBTreeMap;
use unicase_collections::unicase_btree_set::UniCaseBTreeSet;
use crate::uni_codes::{UC_KEY, UC_V_ELSE, UC_V_ELSE_IF, UC_V_FOR, UC_V_IF, UC_V_ONCE, UC_V_PRE};
use crate::{ARG_RE, BIND_RE, DIR_RE, DYNAMIC_ARG_RE, FOR_ALIAS_RE, FOR_ITERATOR_RE, MODIFIER_RE, ON_RE, PROP_BIND_RE, SLOT_RE, STRIP_PARENS_RE, warn};
use crate::filter_parser::parse_filters;
use regex::Regex;
use crate::directives_model::gen_assignment_code;
use crate::helpers::{is_some_and_ref, to_camel, to_hyphen_case};
use crate::util::prepend_modifier_marker;

pub const EMPTY_SLOT_SCOPE_TOKEN: &'static str = "_empty_";

#[derive(Debug)]
pub struct AttrItem {
    pub name: String,
    pub value: String,
    pub dynamic: bool,
    pub quote_type: QuoteType
}

#[derive(Debug)]
pub struct Handler {
    value: String,
    dynamic: bool,
    modifiers: UniCaseBTreeSet,
}

#[derive(Debug)]
pub struct Directive {
    name: String,
    raw_name: String,
    value: String,
    arg: Option<String>,
    is_dynamic_arg: bool,
    modifiers: UniCaseBTreeSet,
}

#[derive(Debug)]
pub struct IfCondition {
    pub exp: Option<String>,
    pub block_id: usize,
}

#[derive(Debug)]
pub struct ASTElement {
    // rs_html_parser_tokens Token
    pub token: Token,

    // TODO: internal helpers, move these somewhere else
    pub is_dev: bool,
    pub new_slot_syntax: bool,

    // extra
    pub forbidden: bool,
    pub pre: bool,
    pub plain: bool,
    pub ignored: UniCaseBTreeSet,
    pub processed: bool,
    pub ref_val: Option<String>,
    pub ref_in_for: bool,

    pub component: bool,
    pub inline_template: bool,

    pub attrs: Vec<AttrItem>,
    pub dynamic_attrs: Vec<AttrItem>,
    pub props: Vec<AttrItem>,

    pub directives: Option<Vec<Directive>>,

    pub events: Option<UniCaseBTreeMap<Vec<Handler>>>,
    pub native_events: Option<UniCaseBTreeMap<Vec<Handler>>>,

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
    pub if_conditions: Option<Vec<IfCondition>>,

    pub once: bool,

    pub slot_name: Option<String>,
    pub slot_target: Option<String>,
    pub slot_target_dynamic: bool,
    pub slot_scope: Option<Box<str>>,
    pub scoped_slots: Option<UniCaseBTreeMap<Rc<RefCell<ASTNode>>>>,
    pub has_bindings: bool
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
        if_conditions: None,
        once: false,
        slot_name: None,
        slot_target: None,
        key: None,

        is_dev,
        ref_in_for: false,
        component: false,
        inline_template: false,
        attrs: vec![],
        scoped_slots: None,
        slot_scope: None,
        dynamic_attrs: vec![],
        slot_target_dynamic: false,
        new_slot_syntax: false,
        has_bindings: false,
        props: vec![],
        directives: None,
        events: None,
        native_events: None,
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
    counter: Cell<usize>,
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
            counter: Cell::new(0),
            root: Rc::clone(&node),
            nodes: Default::default(),
        };

        tree.nodes.insert(0, Rc::clone(&node));

        return tree;
    }

    pub fn create(&self, element: ASTElement, parent_id: usize) -> Rc<RefCell<ASTNode>> {
        let new_id = self.counter.get() + 1;
        let parent = self.get(parent_id).cloned().unwrap();

        let new_node = Rc::new(RefCell::new(ASTNode {
            id: new_id,
            el: element,
            parent: Some(Rc::downgrade(&parent)),
            children: vec![]
        }));

        parent.borrow_mut().children.push(Rc::clone(&new_node));

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
            let new_exp = Some(vif_value.to_string());
            self.el.if_val = new_exp.clone();
            self.add_if_condition(IfCondition {
                exp: new_exp.clone(),
                block_id: self.id,
            });
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

    pub fn get_and_remove_attr_by_regex(&mut self, name: &Regex) -> Option<Box<str>> {
        for (attr_name, attr_value) in self.el.token.attrs.as_ref().unwrap().iter() {
            if name.is_match(attr_name) {
                self.el.ignored.insert(attr_name.clone());

                if let Some((attr_value, _attr_quote)) = attr_value {
                    return Some(attr_value.clone());
                }
            }
        }

        return None;
    }

    pub fn has_raw_attr(
        &self,
        name: &str,
    ) -> bool {
        if let Some(ref attrs) = self.el.token.attrs {
            return attrs.contains_key(name);
        }

        return false;
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

    pub fn add_if_condition(&mut self, if_condition: IfCondition) {
        if self.el.if_conditions.is_none() {
            self.el.if_conditions = Some(vec![]);
        }

        self.el.if_conditions.as_mut().unwrap().push(if_condition);
    }

    pub fn process_if_conditions(&mut self, parent_children: &mut Vec<Rc<RefCell<ASTNode>>>) {
        let prev = find_prev_element(parent_children);
        if let Some(prev_element) = prev {
            if prev_element.borrow().el.if_val.is_some() {
                prev_element.borrow_mut().add_if_condition(IfCondition {
                    exp: self.el.else_if_val.clone(),
                    block_id: self.id,
                });
            }
        } else if self.el.is_dev {
            warn(&format!(
                "v-{} used on element <{}> without corresponding v-if.",
                match &self.el.else_if_val {
                    Some(else_if_val) => format!("else-if=\"{}\"", else_if_val),
                    None => "else".to_string(),
                },
                self.el.token.data
            ));
        }
    }

    pub fn process_element(&mut self, tree: &ASTTree) {
        self.process_key();

        // determine whether this is a plain element after
        // removing structural attributes
        self.el.plain = self.el.key.is_none() && self.el.scoped_slots.is_none() && self.el.token.attrs.is_none();

        self.process_ref();
        self.process_slot_content(tree);
        self.process_slot_outlet();
        self.process_component();

        /* TODO: finish this
           for (let i = 0; i < transforms.length; i++) {
            element = transforms[i](element, options) || element
          }
         */

        self.process_attrs();
    }

    // handle <slot/> outlets
    pub fn process_slot_outlet(&mut self) {
        if self.el.token.data.eq_ignore_ascii_case("slot") {
            let slot_name = self.get_binding_attr("name", false);

            if slot_name.is_empty() {
                self.el.slot_name = Some(slot_name);
            }

            if self.el.is_dev && self.el.key.is_some() {
                println!(
                    "`key` does not work on <slot> because slots are abstract outlets \
                and can possibly expand into multiple elements. \
                Use the key on a wrapping element instead. {}",
                    self.get_raw_binding_attr("key").unwrap_or(&"".into())
                );
            }
        }
    }

    pub fn process_component(&mut self) {
        let binding = self.get_binding_attr("is", false);

        if !binding.is_empty() {
            self.el.component = true;
        }

        if self.get_and_remove_attr("inline-template", false).is_some() {
            self.el.inline_template = true;
        }
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

    pub fn process_slot_content(&mut self, tree: &ASTTree) {
        let is_dev = self.el.is_dev;
        let slot_scope: Option<Box<str>>;

        if self.el.token.data.eq_ignore_ascii_case("template") {
            slot_scope = self.get_and_remove_attr("scope", false).cloned();

            if is_dev && slot_scope.is_some() {
                warn("the \"scope\" attribute for scoped slots have been deprecated and replaced by \"slot-scope\" since 2.5. The new \"slot-scope\" attribute can also be used on plain elements in addition to <template> to denote scoped slots.");
            }
            self.el.slot_scope = if slot_scope.is_some() {
                slot_scope
            } else {
                self.get_and_remove_attr("slot-scope", false).cloned()
            };
        } else {
            slot_scope = self.get_and_remove_attr("slot-scope", false).cloned();

            if slot_scope.is_some() {
                if self.get_and_remove_attr("slot-scope", false).is_some() {
                    if is_dev && self.has_raw_attr("v-for") {
                        warn("Ambiguous combined usage of slot-scope and v-for on <{TODO}> (v-for takes higher priority). Use a wrapper <template> for the scoped slot to make it clearer.");
                    }
                }
            }

            self.el.slot_scope = slot_scope;
        }

        // slot="xxx"
        let slot_target = self.get_and_remove_attr("slot", false).cloned();
        if let Some(slot_target_value) = slot_target {
            self.el.slot_target = if slot_target_value.is_empty() {
                Some("default".to_string())
            } else {
                Some(slot_target_value.to_string())
            };
            self.el.slot_target_dynamic = self.has_raw_attr("slot") || self.has_raw_attr("v-bind:slot");
            // preserve slot as an attribute for native shadow DOM compat
            // only for non-scoped slots.
            if !self.el.token.data.eq_ignore_ascii_case("template") && !self.el.slot_scope.is_some() {
                self.insert_into_attrs("slot", &slot_target_value, QuoteType::NoValue, false);
            }
        }

        // 2.6 v-slot syntax
        if self.el.new_slot_syntax {
            if self.el.token.data.eq_ignore_ascii_case("template") {
                let slot_binding = self.get_and_remove_attr_by_regex(&SLOT_RE);

                if let Some(slot_binding_val) = slot_binding {
                    if is_dev {
                        let slot_target = self.el.slot_target.clone();
                        let slot_scope = self.el.slot_scope.clone();

                        if slot_target.is_some() || slot_scope.is_some() {
                            warn("Unexpected mixed usage of different slot syntaxes. (slot-target, slot-scope)");
                        }
                        if let Some(parent) = self.parent.as_ref().and_then(|parent_weak| parent_weak.upgrade()) {
                            if parent.borrow().is_maybe_component() {
                                warn("<template v-slot> can only appear at the root level inside the receiving component.");
                            }
                        }
                    }
                    let slot_name = get_slot_name(&*slot_binding_val);
                    self.el.slot_target = Some(slot_name.name);
                    self.el.slot_target_dynamic = slot_name.dynamic;
                    self.el.slot_scope = Some(if slot_binding_val.is_empty() { Box::from(EMPTY_SLOT_SCOPE_TOKEN) } else { slot_binding_val.clone() });
                }
            } else {
                let slot_binding = self.get_and_remove_attr_by_regex(&SLOT_RE);

                if let Some(slot_binding_val) = slot_binding {
                    if is_dev {
                        if !self.is_maybe_component() {
                            warn("v-slot can only be used on components or <template>.")
                        }
                        if self.el.slot_scope.is_some() || self.el.slot_target.is_some() {
                            warn("Unexpected mixed usage of different slot syntaxes. (slot-scope, slot)");
                        }
                        if self.el.scoped_slots.is_some() {
                            warn("To avoid scope ambiguity, the default slot should also use <template> syntax when there are other named slots.");
                        }
                    }
                    let slots = if self.el.scoped_slots.is_some() {
                        self.el.scoped_slots.as_mut().unwrap()
                    } else {
                        self.el.scoped_slots = Some(UniCaseBTreeMap::new());
                        self.el.scoped_slots.as_mut().unwrap()
                    };

                    let slot_name = get_slot_name(&*slot_binding_val);
                    let slot_container = tree.create(
                        create_ast_element(Token {
                            kind: OpenTag,
                            data: "template".into(),
                            attrs: None,
                            is_implied: false,
                        }, is_dev),
                        self.id
                    );
                    let mut slot_container_node = slot_container.borrow_mut();

                    slot_container_node.el.slot_target = Some(slot_name.name.to_string());
                    slot_container_node.el.slot_target_dynamic = slot_name.dynamic;

                    // Convert self to a Weak reference
                    let parent = tree.get(self.id).cloned().unwrap();

                    slot_container_node.children = self.children.iter().map(|child| Rc::clone(child)).filter_map(|child_rc| {
                        let mut child = child_rc.borrow_mut();
                        if child.el.slot_scope.is_none() {
                            child.parent = Some(Rc::downgrade(&parent));
                            Some(Rc::clone(&child_rc))
                        } else {
                            None
                        }
                    }).collect::<Vec<_>>();
                    slot_container_node.el.slot_scope = Some(if slot_binding_val.is_empty() { Box::from(EMPTY_SLOT_SCOPE_TOKEN) } else { slot_binding_val.clone() });
                    drop(slot_container_node);
                    slots.insert(slot_name.name.to_string(), slot_container);

                    // remove children as they are returned from scopedSlots now
                    self.children = vec![];
                    // mark el non-plain so data gets generated
                    self.el.plain = false;
                }
            }
        }
    }

    fn insert_into_attrs(&mut self, key: &str, value: &str, quote_type: QuoteType, is_dynamic: bool) {
        self.el.plain = false;

        let item = AttrItem {
            name: key.to_string(),
            value: value.to_string(),
            dynamic: is_dynamic,
            quote_type
        };

        if is_dynamic {
            self.el.attrs.push(item)
        } else {
            self.el.dynamic_attrs.push(item)
        }
    }

    fn insert_into_props(&mut self, key: &str, value: &str, quote_type: QuoteType, is_dynamic: bool) {
        self.el.plain = false;

        let item = AttrItem {
            name: key.to_string(),
            value: value.to_string(),
            dynamic: is_dynamic,
            quote_type
        };
        self.el.attrs.push(item);
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

    pub fn check_for_alias_model(&self, val: &str) {
        if !self.el.is_dev {
            return
        }

        // TODO: Finish variables in warning
        let warn_text =
r#""<{el.tag} v-model="${value}">
You are binding v-model directly to a v-for iteration alias.
This will not be able to modify the v-for source array because
writing to the alias is like modifying a function local variable.
Consider using an array of objects and use v-model on an object property instead.""#;


        if self.el.for_value.is_some() && is_some_and_ref(&self.el.alias, |alias| alias.eq_ignore_ascii_case(val)) {
            warn(warn_text);
            return;
        }

        let mut current_node = self.parent.as_ref().and_then(|parent_weak| parent_weak.upgrade());

        while let Some(node) = current_node {
            if node.borrow().el.for_value.is_some() && is_some_and_ref(&node.borrow().el.alias, |alias| alias.eq_ignore_ascii_case(val)) {
                warn(warn_text);
            }
            current_node = node.borrow().parent.as_ref().and_then(|parent_weak| parent_weak.upgrade());
        }
    }

    // TODO: Finish this
    pub fn is_maybe_component(&self) -> bool {
        self.el.component ||
        self.has_raw_attr("is") ||
        self.has_raw_attr("v-bind:is")
        // !(self.el.token.attrs.attrsMap.is ? isReservedTag(el.attrsMap.is) : isReservedTag(el.tag))
    }
    pub fn process_attrs(&mut self) {
        if self.el.token.attrs.is_none() {
            return;
        }

        // TODO: Get rid off this clone
        let attrs = self.el.token.attrs.clone().unwrap();
        for (orig_name, orig_val) in attrs.iter() {
            self.process_attr(&orig_name, &orig_val);
        }
    }

    pub fn process_attr(&mut self, name: &str, value: &Option<(Box<str>, QuoteType)>) {
        let mut name_str = name.to_string();
        let raw_name = name_str.clone();
        let mut value = value.clone();

        if DIR_RE.is_match(&name_str) {
            // mark element as dynamic
            self.el.has_bindings = true;

            // modifiers

            let mut modifiers_option = parse_modifiers(DIR_RE.replace_all(&name_str, "").as_ref());

            // support .foo shorthand syntax for the .prop modifier
            if PROP_BIND_RE.is_match(&name_str) && modifiers_option.is_some() {
                modifiers_option.as_mut().unwrap().insert("prop");
                name_str = ".".to_string() + &*MODIFIER_RE.replace_all(&name_str[1..], "");
            } else if modifiers_option.is_some() {
                name_str = MODIFIER_RE.replace_all(&name_str, "").to_string();
            }

            if BIND_RE.is_match(&name_str) {
                // v-bind
                name_str = BIND_RE.replace_all(&name_str, "").to_string();
                if let Some(val) = value {
                    value = Some((Box::from(parse_filters(&val)), val.1));
                }

                let is_dynamic = DYNAMIC_ARG_RE.is_match(&name_str);
                if is_dynamic {
                    name_str = name_str[1..name_str.len() - 1].to_string();
                }

                if self.el.is_dev && value.as_ref().is_some_and(|v| v.0.trim().is_empty()) {
                    warn(&format!("The value for a v-bind expression cannot be empty. Found in \"v-bind:{}\"", name_str));
                }

                if let Some(ref modifiers) = modifiers_option.as_ref() {
                    if modifiers.contains("prop") && !is_dynamic {
                        if name_str.eq_ignore_ascii_case("innerHtml") {
                            name_str = "innerHTML".to_string();
                        }
                    }
                    if modifiers.contains("camel") && !is_dynamic {
                        name_str = to_camel(&name_str);
                    }
                    if modifiers.contains("sync") {
                        let sync_gen = if value.is_some() { gen_assignment_code(value.as_ref().unwrap(), "$event") } else { "".to_string() };
                        if !is_dynamic {
                            let camel_case_name = to_camel(&name_str);
                            let hyphen_case_name = to_hyphen_case(&name_str);

                            self.add_handler(
                                &format!("update:{}", camel_case_name),
                                &sync_gen,
                                None,
                                false,
                                false
                            );

                            if hyphen_case_name != camel_case_name {
                                self.add_handler(
                                    &format!("update:{}", hyphen_case_name),
                                    &sync_gen,
                                    None,
                                    false,
                                    false
                                );
                            }
                        } else {
                            // handler w/ dynamic event name
                            self.add_handler(
                                &format!("\"update:\"+({})", name_str),
                                &sync_gen,
                                None,
                                false,
                                true
                            );
                        }
                    }
                }
                let attr_value = if value.is_some() { value.unwrap() } else { ( "".to_string().into_boxed_str(), QuoteType::NoValue) };

                if is_some_and_ref(&modifiers_option, |modifiers| modifiers.contains("prop")) {
                    // TODO: (!el.component && platformMustUseProp(el.tag, el.attrsMap.type, name))
                    self.insert_into_attrs(&name_str, &attr_value.0, attr_value.1, is_dynamic);
                } else {
                    self.insert_into_attrs(&name_str, &attr_value.0, attr_value.1, is_dynamic);
                }

            } else if ON_RE.is_match(&name_str) {
                // v-on
                let attr_value = if value.is_some() { value.unwrap() } else { ( "".to_string().into_boxed_str(), QuoteType::NoValue) };

                name_str = ON_RE.replace_all(&name_str, "").to_string();
                let is_dynamic = DYNAMIC_ARG_RE.is_match(&name_str);
                if is_dynamic {
                    name_str = name_str[1..name_str.len() - 1].to_string();
                }
                self.add_handler(&name_str, &attr_value.0, modifiers_option, false, is_dynamic);
            } else {
                let attr_value = if value.is_some() { value.unwrap() } else { ( "".to_string().into_boxed_str(), QuoteType::NoValue) };

                // normal directives
                name_str = DIR_RE.replace_all(&name_str, "").to_string();
                // parse arg
                let name_clone = name_str.to_string();
                let arg_match = ARG_RE.captures(&name_clone);
                let mut arg = arg_match.and_then(|cap| cap.get(1)).map(|m| m.as_str());
                let mut is_dynamic = false;
                if let Some(arg_val) = arg {
                    name_str = (&name_str[..name_str.len() - arg_val.len() - 1]).to_string();
                    if DYNAMIC_ARG_RE.is_match(arg_val) {
                        arg = Some(&arg_val[1..arg_val.len() - 1]);
                        is_dynamic = true;
                    }
                }
                self.add_directive(
                    &name_str,
                    &raw_name,
                    &(attr_value.0),
                    arg,
                    is_dynamic,
                    modifiers_option
                );
                if self.el.is_dev && name_str.eq_ignore_ascii_case("model") {
                    self.check_for_alias_model(&(attr_value.0));
                }
            }
        } else {
            let attr_value = if value.is_some() { value.unwrap() } else { ( "".to_string().into_boxed_str(), QuoteType::NoValue) };

            // literal attribute
            if self.el.is_dev {
                // TODO: Finish validation
                // let res = parse_text(&value, delimiters);
                // if let Some(res) = res {
                //     warn(
                //         format!("{}=\"{}\": Interpolation inside attributes has been removed. Use v-bind or the colon shorthand instead. For example, instead of <div id=\"{{ val }}\">, use <div :id=\"val\">.", name, value)
                //     );
                // }
            }
            self.insert_into_attrs(&name_str, &(attr_value.0), attr_value.1, false);
            // #6887 firefox doesn't update muted state if set via attribute
            // even immediately after element creation
            // if (
            //     !self.el.component &&
            //         name == "muted" &&
            //         platform_must_use_prop(self.el.tag, self.el.attrs_map.get("type"), &name)
            // ) {
            //     add_prop(self, &name, "true", attrs.get(name).unwrap(), false);
            // }
        }
    }

    pub fn add_handler(
        &mut self,
        name: &str,
        value: &str,
        modifiers: Option<UniCaseBTreeSet>,
        important: bool,
        dynamic: bool,
    ) {
        let mut modifiers = modifiers.unwrap_or(UniCaseBTreeSet::new());
        let mut name = name.to_string();

        if cfg!(debug_assertions) {
            if modifiers.get("prevent").is_some() && modifiers.get("passive").is_some() {
                warn("passive and prevent can't be used together. Passive handler can't prevent default event.");
            }
        }

        if modifiers.remove("right") {
            if dynamic {
                name = format!("({})==='click'?'contextmenu':({})", name, name);
            } else if name == "click" {
                name = "contextmenu".to_string();
            }
        } else if modifiers.remove("middle") {
            if dynamic {
                name = format!("({})==='click'?'mouseup':({})", name, name);
            } else if name == "click" {
                name = "mouseup".to_string();
            }
        }

        if modifiers.remove("capture") {
            name = prepend_modifier_marker('!', &name, dynamic);
        }
        if modifiers.remove("once") {
            name = prepend_modifier_marker('~', &name, dynamic);
        }
        if modifiers.remove("passive") {
            name = prepend_modifier_marker('&', &name, dynamic);
        }

        let events = (if modifiers.remove("native") {
            &mut self.el.native_events
        } else {
            &mut self.el.events
        }).get_or_insert(UniCaseBTreeMap::new());

        let new_handler = Handler {
            value: value.trim().to_string(),
            dynamic,
            modifiers,
        };

        let handlers = events.entry(name).or_insert_with(Vec::new);
        if important {
            handlers.insert(0, new_handler);
        } else {
            handlers.push(new_handler);
        }

        self.el.plain = false;
    }

    pub fn add_directive(
        &mut self,
        name: &str,
        raw_name: &str,
        value: &str,
        arg: Option<&str>,
        is_dynamic_arg: bool,
        modifiers: Option<UniCaseBTreeSet>,
    ) {
        let modifiers = modifiers.unwrap_or(UniCaseBTreeSet::new());

        let directive = Directive {
            name: name.to_string(),
            raw_name: raw_name.to_string(),
            value: value.to_string(),
            arg: arg.map(|arg| arg.to_string()),
            is_dynamic_arg,
            modifiers,
        };

        self.el.directives.get_or_insert(Vec::new()).push(directive);
        self.el.plain = false;
    }
}

fn find_prev_element(children: &mut Vec<Rc<RefCell<ASTNode>>>) -> Option<&Rc<RefCell<ASTNode>>> {
    if children.len() == 0 {
        return None;
    }

    while children.len() > 0 {
        if children[0].borrow().el.token.kind == OpenTag {
            return Some(&children[0]);
        }

        if children[0].borrow().el.is_dev {
            warn(&format!("text \"{}\" between v-if and v-else(-if) will be ignored.", &children[0].borrow().el.token.data));
        }

        children.remove(0);
    }

    return None;
}

fn parse_modifiers(name: &str) -> Option<UniCaseBTreeSet> {
    let captures = MODIFIER_RE.captures_iter(name);

    let mut ret: Option<UniCaseBTreeSet> = None;
    for cap in captures {
        ret.get_or_insert(UniCaseBTreeSet::new()).insert(cap[0][1..].to_string());
    }

    return ret;
}

#[derive(Debug)]
pub struct SlotName {
    name: String,
    dynamic: bool,
}

pub fn get_slot_name(binding: &str) -> SlotName {
    let mut name = SLOT_RE.replace_all(binding, "").to_string();

    if name.is_empty() {
        if !binding.starts_with('#') {
            name = "default".to_string();
        } else {
            // TODO: warn in debug only
            println!("v-slot shorthand syntax requires a slot name: {}", binding);
        }
    }

    if DYNAMIC_ARG_RE.is_match(&name) {
        // dynamic [name]
        SlotName {
            name: name[1..name.len() - 1].to_string(),
            dynamic: true,
        }
    } else {
        // static name
        SlotName {
            name: format!("\"{}\"", name),
            dynamic: false,
        }
    }
}
