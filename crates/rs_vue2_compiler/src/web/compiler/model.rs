use crate::ast_tree::{create_ast_element, ASTElementKind, ASTNode, ASTTree, IfCondition};
use crate::{CompilerOptions, ModuleApi};
use rs_html_parser_tokenizer_tokens::QuoteType;
use rs_html_parser_tokens::Token;
use std::cell::RefCell;
use std::rc::Rc;
use unicase_collections::unicase_btree_map::UniCaseBTreeMap;

pub struct ModelModule {}

fn node_copy(node: &ASTNode, tree: &ASTTree) -> Rc<RefCell<ASTNode>> {
    tree.create(
        create_ast_element(
            Token {
                data: node.el.token.data.clone(),
                attrs: node.el.token.attrs.clone(),
                kind: node.el.token.kind,
                is_implied: node.el.token.is_implied,
            },
            ASTElementKind::Element,
        ),
        node.parent_id,
        node.is_dev,
        node.warn.clone_box(),
    )
}

impl ModuleApi for ModelModule {
    fn transform_node(&self, _node: &mut ASTNode, _options: &CompilerOptions) {}

    fn gen_data(&self, _node: &ASTNode) -> Option<String> {
        None
    }

    fn static_keys(&self) -> Vec<&'static str> {
        vec![]
    }

    fn pre_transform_node(
        &self,
        node: &mut ASTNode,
        tree: &ASTTree,
    ) -> Option<Rc<RefCell<ASTNode>>> {
        if node.el.token.data.as_ref() == "input" {
            if let Some(map) = &node.el.token.attrs {
                if !map.contains_key("v-model") {
                    return None;
                }

                let mut type_binding = None;
                if map.contains_key(":type") || map.contains_key("v-bind:type") {
                    // type_binding = Some(node.get_binding_attr("type", true));
                }
                if type_binding.is_none() && !map.contains_key("type") && map.contains_key("v-bind")
                {
                    let v_bind_attr = map.get("v-bind");

                    if let Some(v_bind_attr) = v_bind_attr {
                        if let Some((v_bind_attr, _)) = v_bind_attr {
                            type_binding = Some(format!("({}).type", v_bind_attr));
                        }
                    }
                }

                if let Some(type_binding) = type_binding {
                    let if_condition = node.get_and_remove_attr("v-if", true);
                    let if_condition_val = if let Some(if_cond) = if_condition {
                        if_cond.value
                    } else {
                        None
                    };
                    let if_condition_extra = if let Some(ref if_condition) = if_condition_val {
                        format!("&&({})", if_condition)
                    } else {
                        String::new()
                    };
                    let has_else = node.get_and_remove_attr("v-else", true).is_some();
                    let else_if_condition = node.get_and_remove_attr("v-else-if", true);

                    // 1. checkbox
                    let branch0_rc = node_copy(node, tree);
                    {
                        let mut branch0 = branch0_rc.borrow_mut();
                        branch0.process_for();
                        branch0
                            .el
                            .token
                            .attrs
                            .get_or_insert(UniCaseBTreeMap::new())
                            .insert(
                                "type",
                                Some(("checkbox".to_string().into_boxed_str(), QuoteType::Double)),
                            );
                        branch0.process_element(tree);
                        branch0.el.processed = true; // prevent it from double-processed
                        branch0.el.if_val = Some(format!(
                            "({})==='checkbox'{}",
                            type_binding, if_condition_extra
                        ));
                        let if_cond = IfCondition {
                            exp: branch0.el.if_val.clone(),
                            block_id: branch0.id,
                        };
                        branch0.add_if_condition(if_cond);

                        // 2. add radio else-if condition
                        let branch1_rc = node_copy(node, tree);
                        let mut branch1 = branch1_rc.borrow_mut();
                        branch1.get_and_remove_attr("v-for", true);
                        branch1
                            .el
                            .token
                            .attrs
                            .get_or_insert(UniCaseBTreeMap::new())
                            .insert(
                                "type",
                                Some(("radio".to_string().into_boxed_str(), QuoteType::Double)),
                            );
                        branch1.process_element(tree);
                        branch0.add_if_condition(IfCondition {
                            exp: Some(format!(
                                "({})==='radio'{}",
                                type_binding, if_condition_extra
                            )),
                            block_id: branch1.id,
                        });

                        // 3. other
                        let branch2_rc = node_copy(node, tree);
                        let mut branch2 = branch2_rc.borrow_mut();
                        branch2.get_and_remove_attr("v-for", true);
                        branch2
                            .el
                            .token
                            .attrs
                            .get_or_insert(UniCaseBTreeMap::new())
                            .insert(
                                ":type",
                                Some((
                                    type_binding.to_string().into_boxed_str(),
                                    QuoteType::Single,
                                )),
                            );
                        branch2.process_element(tree);
                        branch0.add_if_condition(IfCondition {
                            exp: if_condition_val.clone(),
                            block_id: branch2.id,
                        });

                        if has_else {
                            branch0.el.is_else = true;
                        } else if let Some(else_if_condition) = else_if_condition {
                            if let Some(else_if_val) = else_if_condition.value {
                                branch0.el.else_if_val = Some(else_if_val);
                            } else {
                                node.warn.call("empty v-else-if condition");
                            }
                        }
                    }

                    return Some(branch0_rc);
                }
            }
        }

        None
    }
}
