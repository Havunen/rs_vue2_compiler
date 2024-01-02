use crate::ast_tree::{ASTNode, ASTTree};
use crate::text_parser::parse_text;
use crate::{CompilerOptions, ModuleApi};
use std::cell::RefCell;
use std::rc::Rc;

pub struct ClassModule {}

impl ModuleApi for ClassModule {
    fn transform_node(&self, node: &mut ASTNode, options: &CompilerOptions) {
        // Here we would have the equivalent of getAndRemoveAttr and getBindingAttr
        // For simplicity, let's assume we've done that and have the results in static_class and class_binding
        let static_class = node.get_and_remove_attr("class", false);

        if let Some(static_class) = &static_class {
            if let Some(static_class_val) = &static_class.value {
                node.el.static_class = Some(static_class_val.replace(" ", "").to_string());

                if node.is_dev {
                    let parsed = parse_text(static_class_val, &options.delimiters);

                    if parsed.is_some() {
                        node.warn.call(&format!(
                            "class=\"{}\": Interpolation inside attributes has been removed. \
                Use v-bind or the colon shorthand instead. For example, \
                instead of <div class=\"{{ val }}\">, use <div :class=\"val\">.",
                            static_class_val
                        ));
                    }
                }
            }
        }

        let class_binding = node.get_binding_attr("class", true);

        if !class_binding.is_empty() {
            node.el.class_binding = Some(class_binding);
        }
    }

    fn gen_data(&self, node: &ASTNode) -> Option<String> {
        let mut data = String::new();

        if let Some(static_class) = &node.el.static_class {
            data += &format!("staticClass:{},", static_class);
        }

        if let Some(class_binding) = &node.el.class_binding {
            data += &format!("class:{},", class_binding);
        }

        Some(data)
    }

    fn static_keys(&self) -> Vec<&'static str> {
        vec!["staticClass"]
    }

    fn pre_transform_node(
        &self,
        _node: &mut ASTNode,
        _tree: &mut ASTTree,
        _options: &CompilerOptions,
    ) -> Option<Rc<RefCell<ASTNode>>> {
        None
    }
}
