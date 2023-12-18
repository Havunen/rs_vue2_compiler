use std::cell::{Ref, RefMut};
use std::rc::Rc;
use crate::ast_tree::{ASTElement, ASTNode, ASTTree};
use crate::uni_codes::UC_KEY;
use crate::warn;

pub fn process_element(node: RefMut<ASTNode>) {
    process_key(node);
}

pub fn process_key(mut node: RefMut<ASTNode>) {
    let exp = node.get_binding_attr(&UC_KEY, false);

    if !exp.is_empty() {
        if node.el.is_dev {
            if node.el.token.data.eq_ignore_ascii_case("template") {
                // self.get_raw_binding_attr(&UC_KEY).unwrap_or("".into()).to_string().as_str())
                warn("<template> cannot be keyed. Place the key on real elements instead. {}");
            }

            let has_iterator_1 = node.el.iterator1.is_some() && node.el.iterator1.as_ref().unwrap().eq(&exp);
            let has_iterator_2 = node.el.iterator2.is_some() && node.el.iterator2.as_ref().unwrap().eq(&exp);

            if node.el.for_value.is_some() {
                if has_iterator_1 || has_iterator_2 {
                    {
                        if let Some(parent) = node.parent.as_ref().unwrap().upgrade() {
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

            node.el.key = Some(exp);
        }
    }
}
