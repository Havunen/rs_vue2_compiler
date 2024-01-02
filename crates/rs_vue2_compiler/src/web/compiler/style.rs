use crate::ast_tree::{ASTNode, ASTTree};
use crate::text_parser::parse_text;
use crate::util::parse_style_text;
use crate::{CompilerOptions, ModuleApi};
use std::cell::RefCell;
use std::rc::Rc;

pub struct StyleModule {}

impl ModuleApi for StyleModule {
    fn transform_node(&self, node: &mut ASTNode, options: &CompilerOptions) {
        let static_style_attr_opt = node.get_and_remove_attr("style", false);

        if let Some(static_style_attr) = &static_style_attr_opt {
            if let Some(static_style) = &static_style_attr.value {
                if node.is_dev {
                    let res = parse_text(static_style, &options.delimiters);
                    if res.is_some() {
                        node.warn.call(&format!(
                            "style=\"{}\": Interpolation inside attributes has been removed. \
                        Use v-bind or the colon shorthand instead. For example, \
                        instead of <div style=\"{{ val }}\">, use <div :style=\"val\">.",
                            static_style
                        ));
                    }
                }
                let result = serde_json::to_string(&parse_style_text(static_style));
                match result {
                    Ok(result) => {
                        node.el.static_style = Some(result);
                    }
                    Err(err) => {
                        node.el.static_style = None;
                        node.warn.call(&format!("Failed to parse style: {}", err));
                    }
                }
            }
        }

        let style_binding = node.get_binding_attr("style", false);
        if !style_binding.is_empty() {
            node.el.style_binding = Some(style_binding);
        }
    }

    fn gen_data(&self, node: &ASTNode) -> Option<String> {
        let mut data = String::new();

        if let Some(static_style) = &node.el.static_style {
            data += &format!("staticStyle:{},", static_style);
        }

        if let Some(style_binding) = &node.el.style_binding {
            data += &format!("style:({}),", style_binding);
        }

        Some(data)
    }

    fn static_keys(&self) -> Vec<&'static str> {
        vec!["staticStyle"]
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
