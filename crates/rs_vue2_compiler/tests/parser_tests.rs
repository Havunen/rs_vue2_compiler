#[cfg(test)]
mod tests {
    use rs_vue2_compiler::ast_tree::ASTTree;
    use rs_vue2_compiler::{CompilerOptions, VueParser, WhitespaceHandling};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn parse(template: &str) -> (ASTTree, Rc<RefCell<Vec<String>>>) {
        let warnings = Rc::new(RefCell::new(Vec::new()));
        let warnings_clone = Rc::clone(&warnings);

        let mut parser = VueParser::new(CompilerOptions {
            dev: true,
            is_ssr: false,
            preserve_comments: false,
            whitespace_handling: WhitespaceHandling::Condense,
            is_pre_tag: None,
            get_namespace: None,
            warn: Some(Box::new(move |msg: &str| {
                warnings_clone.borrow_mut().push(msg.to_string());
            })),
        });

        (parser.parse(template), warnings)
    }

    #[test]
    fn simple_element() {
        let (ast, _warnings) = parse("<h1>hello world</h1>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("h1"));
        assert_eq!(root.el.plain, true);
        assert_eq!(
            root.children[0].borrow().el.token.data,
            Box::from("hello world")
        );
    }

    #[test]
    fn interpolation_in_element() {
        let (ast, _warnings) = parse("<h1>{{msg}}</h1>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("h1"));
        assert_eq!(root.el.plain, true);
        assert_eq!(
            root.children[0].borrow().el.expression.clone().unwrap(),
            String::from("_s(msg)")
        );
    }

    #[test]
    fn child_elements() {
        let (ast, _warnings) = parse("<ul><li>hello world</li></ul>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("ul"));
        assert_eq!(root.el.plain, true);

        let child = root.children[0].borrow();
        assert_eq!(child.el.token.data, Box::from("li"));
        assert_eq!(child.el.plain, true);
        assert_eq!(
            child.children[0].borrow().el.token.data,
            Box::from("hello world")
        );

        let parent = child.parent.as_ref().unwrap().upgrade().unwrap();
        assert_eq!(Rc::ptr_eq(&parent, &wrapper.children[0]), true);
    }

    #[test]
    fn unary_element() {
        let (ast, _warnings) = parse("<hr>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("hr"));
        assert_eq!(root.el.plain, true);
        assert_eq!(root.children.len(), 0);
    }

    #[test]
    fn svg_element() {
        let (ast, _warnings) = parse("<svg><text>hello world</text></svg>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("svg"));
        assert_eq!(root.el.ns, Some("svg"));
        assert_eq!(root.el.plain, true);

        let child = root.children[0].borrow();
        assert_eq!(child.el.token.data, Box::from("text"));
        assert_eq!(
            child.children[0].borrow().el.token.data,
            Box::from("hello world")
        );

        let parent = child.parent.as_ref().unwrap().upgrade().unwrap();
        assert_eq!(Rc::ptr_eq(&parent, &wrapper.children[0]), true);
    }

    #[test]
    fn camel_case_element() {
        let (ast, _warnings) = parse("<MyComponent><p>hello world</p></MyComponent>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("MyComponent"));
        assert_eq!(root.el.plain, true);

        let child = root.children[0].borrow();
        assert_eq!(child.el.token.data, Box::from("p"));
        assert_eq!(child.el.plain, true);
        assert_eq!(
            child.children[0].borrow().el.token.data,
            Box::from("hello world")
        );

        let parent = child.parent.as_ref().unwrap().upgrade().unwrap();
        assert_eq!(Rc::ptr_eq(&parent, &wrapper.children[0]), true);
    }

    #[test]
    fn forbidden_element() {
        // style
        let (style_ast, _warnings) = parse("<style>error { color: red; }</style>");

        let wrapper = style_ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("style"));
        assert_eq!(root.el.plain, true);
        assert_eq!(root.el.forbidden, true);
        assert_eq!(
            root.children[0].borrow().el.token.data,
            Box::from("error { color: red; }")
        );

        // script
        let (script_ast, _script_warnings) =
            parse("<script type=\"text/javascript\">alert(\"hello world!\")</script>");

        let wrapper = script_ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("script"));
        assert_eq!(root.el.plain, false);
        assert_eq!(root.el.forbidden, true);
        assert_eq!(
            root.children[0].borrow().el.token.data,
            Box::from("alert(\"hello world!\")")
        );
    }

    #[test]
    fn not_contain_root_element() {
        let (ast, warnings) = parse("hello world");

        let wrapper = ast.wrapper.borrow();
        assert_eq!(wrapper.children.len(), 0);
        assert_eq!(warnings.borrow().len(), 1 as usize);
        assert_eq!(
            warnings.borrow()[0],
            "Component template requires a root element, rather than just text."
        );
    }

    #[test]
    fn warn_text_before_root_element() {
        let (ast, warnings) = parse("before root {{ interpolation }}<div></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();

        assert_eq!(root.children.len(), 0);
        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(
            warnings.borrow()[0],
            "text \"before root {{ interpolation }}\" outside root element will be ignored."
        );
    }

    #[test]
    fn warn_text_after_root_element() {
        let (ast, warnings) = parse("<div></div>after root {{ interpolation }}");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();

        assert_eq!(root.children.len(), 0);
        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(
            warnings.borrow()[0],
            "text \"after root {{ interpolation }}\" outside root element will be ignored."
        );
    }

    #[test]
    fn warn_multiple_root_elements() {
        let (ast, warnings) = parse("<div></div><span></span>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("div"));
        assert_eq!(root.children.len(), 0);
        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(warnings.borrow()[0], "Component template should contain exactly one root element. If you are using v-if on multiple elements, use v-else-if to chain them instead.");
    }

    // Condensing white space could be moved to the html parser
    #[test]
    fn remove_duplicate_whitespace_text_nodes_caused_by_comments() {
        let (ast, _warnings) = parse("<div><a></a> <!----> <a></a></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.children.len(), 3);

        let child_1 = root.children[0].borrow();
        assert_eq!(child_1.el.token.data, Box::from("a"));

        let child_2 = root.children[1].borrow();
        assert_eq!(child_2.el.token.data, Box::from(" "));

        let child_3 = root.children[2].borrow();
        assert_eq!(child_3.el.token.data, Box::from("a"));
    }

    #[test]
    fn forbidden_element_2() {
        // style
        let (style_ast, _style_warnings) = parse("<style>error { color: red; }</style>");

        let wrapper = style_ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("style"));
        assert_eq!(root.el.plain, true);
        assert_eq!(root.el.forbidden, true);
        assert_eq!(
            root.children[0].borrow().el.token.data,
            Box::from("error { color: red; }")
        );

        // script
        let (script_ast, _script_warnings) =
            parse("<script type=\"text/javascript\">alert(\"hello world!\")</script>");

        let wrapper = script_ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("script"));
        assert_eq!(root.el.plain, false);
        assert_eq!(root.el.forbidden, true);
        assert_eq!(
            root.children[0].borrow().el.token.data,
            Box::from("alert(\"hello world!\")")
        );
    }

    #[test]
    fn remove_text_nodes_between_v_if_conditions() {
        let (ast, warnings) = parse("<div><foo v-if=\"1\"></foo> <section v-else-if=\"2\"></section> <article v-else></article> <span></span></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.children.len(), 3);

        let child_1 = root.children[0].borrow();
        assert_eq!(child_1.el.token.data, Box::from("foo"));
        assert_eq!(child_1.el.if_conditions.as_ref().unwrap().len(), 3);

        let if_conditions = child_1.el.if_conditions.as_ref().unwrap();
        assert_eq!(if_conditions[0].block_id, 2);
        assert_eq!(if_conditions[0].exp.as_ref().unwrap(), "1");
        assert_eq!(if_conditions[1].block_id, 4);
        assert_eq!(if_conditions[1].exp.as_ref().unwrap(), "2");
        assert_eq!(if_conditions[2].block_id, 6);
        assert_eq!(if_conditions[2].exp, None);

        let child_2 = root.children[1].borrow();
        assert_eq!(child_2.el.token.data, Box::from(" "));

        let child_3 = root.children[2].borrow();
        assert_eq!(child_3.el.token.data, Box::from("span"));

        assert_eq!(warnings.borrow().len(), 0);
    }

    #[test]
    fn warn_non_whitespace_text_between_v_if_conditions() {
        let (ast, warnings) = parse("<div><div v-if=\"1\"></div> foo <div v-else></div></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.children.len(), 1);
        let the_div = root.children[0].borrow();
        assert_eq!(the_div.el.if_conditions.as_ref().unwrap().len(), 2);

        let if_conditions = the_div.el.if_conditions.as_ref().unwrap();
        assert_eq!(if_conditions[0].block_id, 2);
        assert_eq!(if_conditions[0].exp.as_ref().unwrap(), "1");
        assert_eq!(if_conditions[1].block_id, 4);
        assert_eq!(if_conditions[1].exp, None);

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(warnings.borrow()[0], "text \"foo\" between v-if and v-else(-if) will be ignored.");
    }

    #[test]
    fn not_warn_2_root_elements_with_v_if_and_v_else() {
        let (ast, warnings) = parse("<div v-if=\"1\"></div><div v-else></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();

        assert_eq!(root.el.if_conditions.as_ref().unwrap().len(), 2);

        assert_eq!(warnings.borrow().len(), 0);
    }

    #[test]
    fn not_warn_3_root_elements_with_v_if_v_else_if_and_v_else() {
        let (ast, warnings) = parse("<div v-if=\"1\"></div><div v-else-if=\"2\"></div><div v-else></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();

        assert_eq!(root.el.if_conditions.as_ref().unwrap().len(), 3);

        let if_conditions = root.el.if_conditions.as_ref().unwrap();
        assert_eq!(if_conditions[0].block_id, 1);
        assert_eq!(if_conditions[0].exp.as_ref().unwrap(), "1");
        assert_eq!(if_conditions[1].block_id, 2);
        assert_eq!(if_conditions[1].exp.as_ref().unwrap(), "2");
        assert_eq!(if_conditions[2].block_id, 3);
        assert_eq!(if_conditions[2].exp, None);

        assert_eq!(warnings.borrow().len(), 0);
    }

    #[test]
    fn not_warn_2_root_elements_with_v_if_and_v_else_on_separate_lines() {
        let (ast, warnings) = parse("<div v-if=\"1\"></div>\n<div v-else></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();

        assert_eq!(root.el.if_conditions.as_ref().unwrap().len(), 2);

        assert_eq!(warnings.borrow().len(), 0);
    }
}
