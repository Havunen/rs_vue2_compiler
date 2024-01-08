#[cfg(test)]
mod tests {
    use rs_vue2_compiler::ast_tree::ASTTree;
    use rs_vue2_compiler::web::compiler::class::ClassModule;
    use rs_vue2_compiler::web::compiler::model::ModelModule;
    use rs_vue2_compiler::web::compiler::style::StyleModule;
    use rs_vue2_compiler::{CompilerOptions, VueParser, WhitespaceHandling};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn parse(template: &str) -> (ASTTree, Rc<RefCell<Vec<String>>>) {
        let warnings = Rc::new(RefCell::new(Vec::new()));
        let warnings_clone = Rc::clone(&warnings);
        let options = CompilerOptions {
            dev: true,
            is_ssr: false,
            v_bind_prop_short_hand: false,
            preserve_comments: false,
            whitespace_handling: WhitespaceHandling::Condense,
            new_slot_syntax: true,
            is_pre_tag: None,
            get_namespace: None,
            warn: Some(Box::new(move |msg: &str| {
                warnings_clone.borrow_mut().push(msg.to_string());
            })),
            delimiters: None,
            modules: Some(vec![
                Box::new(ClassModule {}),
                Box::new(ModelModule {}),
                Box::new(StyleModule {}),
            ]),
        };
        let mut parser = VueParser::new(&options);

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
        assert_eq!(warnings.borrow().len(), 1);
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
        assert_eq!(
            warnings.borrow()[0],
            "text \"foo\" between v-if and v-else(-if) will be ignored."
        );
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
        let (ast, warnings) =
            parse("<div v-if=\"1\"></div><div v-else-if=\"2\"></div><div v-else></div>");

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

    #[test]
    fn not_warn_3_or_more_root_elements_with_v_if_v_else_if_and_v_else_on_separate_lines() {
        // Test with 3 root elements
        let (ast, warnings) =
            parse("<div v-if=\"1\"></div>\n<div v-else-if=\"2\"></div>\n<div v-else></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();

        assert_eq!(root.el.if_conditions.as_ref().unwrap().len(), 3);
        assert_eq!(warnings.borrow().len(), 0);

        // Test with 5 root elements
        let (ast, warnings) = parse("<div v-if=\"1\"></div>\n<div v-else-if=\"2\"></div>\n<div v-else-if=\"3\"></div>\n<div v-else-if=\"4\"></div>\n<div v-else></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();

        assert_eq!(root.el.if_conditions.as_ref().unwrap().len(), 5);
        assert_eq!(warnings.borrow().len(), 0);
    }

    #[test]
    fn generate_correct_ast_for_2_root_elements_with_v_if_and_v_else_on_separate_lines() {
        let (ast, _warnings) = parse("<div v-if=\"1\"></div>\n<p v-else></p>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();

        assert_eq!(root.el.token.data, Box::from("div"));

        let if_conditions = root.el.if_conditions.as_ref().unwrap();
        assert_eq!(if_conditions[1].block_id, 2);

        let p_node = ast.get(if_conditions[1].block_id).unwrap().borrow();
        assert_eq!(p_node.el.token.data, Box::from("p"));
    }

    #[test]
    fn generate_correct_ast_for_3_or_more_root_elements_with_v_if_and_v_else_on_separate_lines() {
        // Test with 3 root elements
        let (ast, _warnings) =
            parse("<div v-if=\"1\"></div>\n<span v-else-if=\"2\"></span>\n<p v-else></p>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();

        assert_eq!(root.el.token.data, Box::from("div"));

        let if_conditions = root.el.if_conditions.as_ref().unwrap();
        assert_eq!(if_conditions[0].block_id, 1);
        assert_eq!(if_conditions[1].block_id, 2);
        assert_eq!(if_conditions[2].block_id, 3);

        let div_node = ast.get(if_conditions[0].block_id).unwrap().borrow();
        assert_eq!(div_node.el.token.data, Box::from("div"));

        let span_node = ast.get(if_conditions[1].block_id).unwrap().borrow();
        assert_eq!(span_node.el.token.data, Box::from("span"));

        let p_node = ast.get(if_conditions[2].block_id).unwrap().borrow();
        assert_eq!(p_node.el.token.data, Box::from("p"));

        // Test with 5 root elements
        let (ast, _warnings) = parse("<div v-if=\"1\"></div>\n<span v-else-if=\"2\"></span>\n<div v-else-if=\"3\"></div>\n<span v-else-if=\"4\"></span>\n<p v-else></p>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();

        assert_eq!(root.el.token.data, Box::from("div"));

        let if_conditions = root.el.if_conditions.as_ref().unwrap();
        assert_eq!(if_conditions[0].block_id, 1);
        assert_eq!(if_conditions[1].block_id, 2);
        assert_eq!(if_conditions[2].block_id, 3);
        assert_eq!(if_conditions[3].block_id, 4);
        assert_eq!(if_conditions[4].block_id, 5);

        let div_node = ast.get(if_conditions[0].block_id).unwrap().borrow();
        assert_eq!(div_node.el.token.data, Box::from("div"));

        let span_node = ast.get(if_conditions[1].block_id).unwrap().borrow();
        assert_eq!(span_node.el.token.data, Box::from("span"));

        let div_node_2 = ast.get(if_conditions[2].block_id).unwrap().borrow();
        assert_eq!(div_node_2.el.token.data, Box::from("div"));

        let span_node_2 = ast.get(if_conditions[3].block_id).unwrap().borrow();
        assert_eq!(span_node_2.el.token.data, Box::from("span"));

        let p_node = ast.get(if_conditions[4].block_id).unwrap().borrow();
        assert_eq!(p_node.el.token.data, Box::from("p"));
    }

    #[test]
    fn warn_2_root_elements_with_v_if() {
        let (_ast, warnings) = parse("<div v-if=\"1\"></div><div v-if=\"2\"></div>");

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(warnings.borrow()[0], "Component template should contain exactly one root element. If you are using v-if on multiple elements, use v-else-if to chain them instead.");
    }

    #[test]
    fn warn_3_root_elements_with_v_if_and_v_else_on_first_2() {
        let (_ast, warnings) = parse("<div v-if=\"1\"></div><div v-else></div><div></div>");

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(warnings.borrow()[0], "Component template should contain exactly one root element. If you are using v-if on multiple elements, use v-else-if to chain them instead.");
    }

    #[test]
    fn warn_4_root_elements_with_v_if_v_else_if_and_v_else_on_first_2() {
        let (_ast, warnings) =
            parse("<div v-if=\"1\"></div><div v-else-if></div><div v-else></div><div></div>");

        assert_eq!(warnings.borrow().len(), 3);
        assert_eq!(warnings.borrow()[0], "Missing v-else-if expression.");
        assert_eq!(warnings.borrow()[1], "Component template should contain exactly one root element. If you are using v-if on multiple elements, use v-else-if to chain them instead.");
        assert_eq!(warnings.borrow()[2], "Component template should contain exactly one root element. If you are using v-if on multiple elements, use v-else-if to chain them instead.");
    }

    #[test]
    fn warn_2_root_elements_with_v_if_and_v_else_with_v_for_on_2nd() {
        let (_ast, warnings) = parse("<div v-if=\"1\"></div><div v-else v-for=\"i in [1]\"></div>");

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(warnings.borrow()[0], "Cannot use v-for on stateful component root element because it renders multiple elements.");
    }

    #[test]
    fn warn_2_root_elements_with_v_if_and_v_else_if_with_v_for_on_2nd() {
        let (_ast, warnings) =
            parse("<div v-if=\"1\"></div><div v-else-if=\"2\" v-for=\"i in [1]\"></div>");

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(warnings.borrow()[0], "Cannot use v-for on stateful component root element because it renders multiple elements.");
    }

    #[test]
    fn warn_template_as_root_element() {
        let (_ast, warnings) = parse("<template></template>");

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(warnings.borrow()[0], "Cannot use <template> as component root element because it may contain multiple nodes.");
    }

    #[test]
    fn warn_slot_as_root_element() {
        let (_ast, warnings) = parse("<slot></slot>");

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(
            warnings.borrow()[0],
            "Cannot use <slot> as component root element because it may contain multiple nodes."
        );
    }

    #[test]
    fn warn_v_for_on_root_element() {
        let (_ast, warnings) = parse("<div v-for=\"item in items\"></div>");

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(warnings.borrow()[0], "Cannot use v-for on stateful component root element because it renders multiple elements.");
    }

    #[test]
    fn warn_template_key() {
        let (_ast, warnings) =
            parse("<div><template v-for=\"i in 10\" :key=\"i\"></template></div>");

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(
            warnings.borrow()[0],
            "<template> cannot be keyed. Place the key on real elements instead. key was i"
        );
    }

    #[test]
    fn warn_the_child_of_the_transition_group_component_has_sequential_index() {
        let (_ast, warnings) = parse("<div><transition-group><i v-for=\"(o, i) of arr\" :key=\"i\"></i></transition-group></div>");

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(warnings.borrow()[0], "Do not use v-for index as key on <transition-group> children,\nthis is the same as not using keys.");
    }

    #[test]
    fn v_pre_directive() {
        let (ast, _warnings) = parse("<div v-pre id=\"message1\"><p>{{msg}}</p></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.pre, true);
        assert_eq!(root.el.attrs[0].name, "id");
        assert_eq!(root.el.attrs[0].value, Some("message1".to_string()));
        assert_eq!(
            root.children[0].borrow().children[0].borrow().el.token.data,
            Box::from("{{msg}}")
        );
    }

    #[test]
    fn v_pre_directive_should_leave_template_in_dom() {
        let (ast, _warnings) = parse(
            "<div v-pre id=\"message1\"><template id=\"template1\"><p>{{msg}}</p></template></div>",
        );

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.pre, true);
        assert_eq!(root.el.attrs.len(), 2); // should be 1 but we can probably deal with it later
        assert_eq!(root.el.attrs[0].name, "id");
        assert_eq!(root.el.attrs[0].value, Some("message1".to_string()));
        assert_eq!(root.children[0].borrow().el.attrs[0].name, "id");
        assert_eq!(
            root.children[0].borrow().el.attrs[0].value,
            Some("template1".to_string())
        );
    }

    #[test]
    fn v_for_directive_basic_syntax() {
        let (ast, _warnings) = parse("<ul><li v-for=\"item in items\"></li></ul>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        let list_item = root.children[0].borrow();

        assert_eq!(list_item.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(list_item.el.alias.as_ref().unwrap(), "item");
    }

    #[test]
    fn v_for_directive_iteration_syntax() {
        let (ast, _warnings) = parse("<ul><li v-for=\"(item, index) in items\"></li></ul>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        let list_item = root.children[0].borrow();

        assert_eq!(list_item.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(list_item.el.alias.as_ref().unwrap(), "item");
        assert_eq!(list_item.el.iterator1.as_ref().unwrap(), "index");
        assert_eq!(list_item.el.iterator2, None);
    }

    #[test]
    fn v_for_directive_iteration_syntax_multiple() {
        let (ast, _warnings) = parse("<ul><li v-for=\"(item, key, index) in items\"></li></ul>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        let list_item = root.children[0].borrow();

        assert_eq!(list_item.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(list_item.el.alias.as_ref().unwrap(), "item");
        assert_eq!(list_item.el.iterator1.as_ref().unwrap(), "key");
        assert_eq!(list_item.el.iterator2.as_ref().unwrap(), "index");
    }

    #[test]
    fn v_for_directive_key() {
        let (ast, _warnings) =
            parse("<ul><li v-for=\"item in items\" :key=\"item.uid\"></li></ul>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        let list_item = root.children[0].borrow();
        assert_eq!(list_item.el.token.data, "li".into());
        assert_eq!(list_item.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(list_item.el.alias.as_ref().unwrap(), "item");
        assert_eq!(list_item.el.key.as_ref().unwrap(), "item.uid");
    }

    #[test]
    fn v_for_directive_destructuring() {
        let (ast, _warnings) = parse("<ul><li v-for=\"{ foo } in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "{ foo }");

        // with paren
        let (ast, _warnings) = parse("<ul><li v-for=\"({ foo }) in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "{ foo }");

        // multi-var destructuring
        let (ast, _warnings) = parse("<ul><li v-for=\"{ foo, bar, baz } in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "{ foo, bar, baz }");

        // multi-var destructuring with paren
        let (ast, _warnings) = parse("<ul><li v-for=\"({ foo, bar, baz }) in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "{ foo, bar, baz }");

        // with index
        let (ast, _warnings) = parse("<ul><li v-for=\"({ foo }, i) in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "{ foo }");
        assert_eq!(li_ast.el.iterator1.as_ref().unwrap(), "i");

        // with key + index
        let (ast, _warnings) = parse("<ul><li v-for=\"({ foo }, i, j) in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "{ foo }");
        assert_eq!(li_ast.el.iterator1.as_ref().unwrap(), "i");
        assert_eq!(li_ast.el.iterator2.as_ref().unwrap(), "j");

        // multi-var destructuring with index
        let (ast, _warnings) =
            parse("<ul><li v-for=\"({ foo, bar, baz }, i) in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "{ foo, bar, baz }");
        assert_eq!(li_ast.el.iterator1.as_ref().unwrap(), "i");

        // array
        let (ast, _warnings) = parse("<ul><li v-for=\"[ foo ] in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "[ foo ]");

        // multi-array
        let (ast, _warnings) = parse("<ul><li v-for=\"[ foo, bar, baz ] in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "[ foo, bar, baz ]");

        // array with paren
        let (ast, _warnings) = parse("<ul><li v-for=\"([ foo ]) in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "[ foo ]");

        // multi-array with paren
        let (ast, _warnings) = parse("<ul><li v-for=\"([ foo, bar, baz ]) in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "[ foo, bar, baz ]");

        // array with index
        let (ast, _warnings) = parse("<ul><li v-for=\"([ foo ], i) in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "[ foo ]");
        assert_eq!(li_ast.el.iterator1.as_ref().unwrap(), "i");

        // array with key + index
        let (ast, _warnings) = parse("<ul><li v-for=\"([ foo ], i, j) in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "[ foo ]");
        assert_eq!(li_ast.el.iterator1.as_ref().unwrap(), "i");
        assert_eq!(li_ast.el.iterator2.as_ref().unwrap(), "j");

        // multi-array with paren
        let (ast, _warnings) = parse("<ul><li v-for=\"([ foo, bar, baz ]) in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "[ foo, bar, baz ]");

        // multi-array with index
        let (ast, _warnings) =
            parse("<ul><li v-for=\"([ foo, bar, baz ], i) in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "[ foo, bar, baz ]");
        assert_eq!(li_ast.el.iterator1.as_ref().unwrap(), "i");

        // nested
        let (ast, _warnings) = parse(
            "<ul><li v-for=\"({ foo, bar: { baz }, qux: [ n ] }, i, j) in items\"></li></ul>",
        );
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(
            li_ast.el.alias.as_ref().unwrap(),
            "{ foo, bar: { baz }, qux: [ n ] }"
        );
        assert_eq!(li_ast.el.iterator1.as_ref().unwrap(), "i");
        assert_eq!(li_ast.el.iterator2.as_ref().unwrap(), "j");

        // array nested
        let (ast, _warnings) =
            parse("<ul><li v-for=\"([ foo, { bar }, baz ], i, j) in items\"></li></ul>");
        let binding = ast.wrapper.borrow();
        let ul_ast = binding.children[0].borrow();
        let li_ast = ul_ast.children[0].borrow();
        assert_eq!(li_ast.el.for_value.as_ref().unwrap(), "items");
        assert_eq!(li_ast.el.alias.as_ref().unwrap(), "[ foo, { bar }, baz ]");
        assert_eq!(li_ast.el.iterator1.as_ref().unwrap(), "i");
        assert_eq!(li_ast.el.iterator2.as_ref().unwrap(), "j");
    }

    #[test]
    fn v_for_directive_invalid_syntax() {
        let (_ast, warnings) = parse("<ul><li v-for=\"item into items\"></li></ul>");

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(warnings.borrow()[0], "Invalid v-for expression");
    }

    #[test]
    fn v_if_directive_syntax() {
        let (ast, _warnings) = parse("<p v-if=\"show\">hello world</p>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.if_val.as_ref().unwrap(), "show");
        assert_eq!(
            root.el.if_conditions.as_ref().unwrap()[0]
                .exp
                .as_ref()
                .unwrap(),
            "show"
        );
    }

    #[test]
    fn v_else_if_directive_syntax() {
        let (ast, _warnings) = parse("<div><p v-if=\"show\">hello</p><span v-else-if=\"2\">elseif</span><p v-else>world</p></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        let if_ast = root.children[0].borrow();
        let conditions_ast = if_ast.el.if_conditions.as_ref().unwrap();

        assert_eq!(conditions_ast.len(), 3);
        assert_eq!(conditions_ast[0].block_id, 2);
        assert_eq!(conditions_ast[0].exp.as_ref().unwrap(), "show");
        assert_eq!(conditions_ast[1].block_id, 4);
        assert_eq!(conditions_ast[1].exp.as_ref().unwrap(), "2");
        assert_eq!(conditions_ast[2].block_id, 6);
        assert_eq!(conditions_ast[2].exp, None);
    }

    #[test]
    fn v_else_directive_syntax() {
        let (ast, _warnings) = parse("<div><p v-if=\"show\">hello</p><p v-else>world</p></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        let if_ast = root.children[0].borrow();
        let conditions_ast = if_ast.el.if_conditions.as_ref().unwrap();

        assert_eq!(conditions_ast.len(), 2);
        assert_eq!(conditions_ast[0].block_id, 2);
        assert_eq!(conditions_ast[0].exp.as_ref().unwrap(), "show");
        assert_eq!(conditions_ast[1].block_id, 4);
        assert_eq!(conditions_ast[1].exp, None);
    }

    #[test]
    fn v_else_if_directive_invalid_syntax() {
        let (_ast, warnings) = parse("<div><p v-else-if=\"1\">world</p></div>");

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(
            warnings.borrow()[0],
            "v-else-if=\"1\" used on element <p> without corresponding v-if."
        );
    }

    #[test]
    fn v_else_directive_invalid_syntax() {
        let (_ast, warnings) = parse("<div><p v-else>world</p></div>");

        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(
            warnings.borrow()[0],
            "v-else used on element <p> without corresponding v-if."
        );
    }

    #[test]
    fn v_once_directive_syntax() {
        let (ast, _warnings) = parse("<p v-once>world</p>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.once, true);
    }

    #[test]
    fn slot_tag_single_syntax() {
        let (ast, _warnings) = parse("<div><slot></slot></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        let slot = root.children[0].borrow();
        assert_eq!(slot.el.token.data, Box::from("slot"));
        assert_eq!(slot.el.slot_name, None);
    }

    #[test]
    fn slot_tag_named_syntax() {
        let (ast, _warnings) = parse("<div><slot name=\"one\">hello world</slot></div>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        let slot = root.children[0].borrow();
        assert_eq!(slot.el.token.data, Box::from("slot"));
        assert_eq!(slot.el.slot_name.as_ref().unwrap(), "one");
    }

    #[test]
    fn slot_target() {
        let (ast, _warnings) = parse("<p slot=\"one\">hello world</p>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.slot_target.as_ref().unwrap(), "one");
    }

    #[test]
    fn component_properties() {
        let (ast, _warnings) = parse("<my-component :msg=\"hello\"></my-component>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.attrs[0].name, "msg");
        assert_eq!(root.el.attrs[0].value, Some("hello".to_string()));
    }

    #[test]
    fn component_is_attribute() {
        let (ast, _warnings) = parse("<my-component is=\"component1\"></my-component>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.component.as_ref().unwrap(), "component1");
    }

    #[test]
    fn component_inline_template_attribute() {
        let (ast, _warnings) = parse("<my-component inline-template>hello world</my-component>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.inline_template, true);
    }

    #[test]
    fn class_binding() {
        // static
        let (ast1, _warnings1) = parse("<p class=\"class1\">hello world</p>");
        let wrapper1 = ast1.wrapper.borrow();
        let root1 = wrapper1.children[0].borrow();
        assert_eq!(root1.el.static_class.as_ref().unwrap(), "class1");

        // dynamic
        let (ast2, _warnings2) = parse("<p :class=\"class1\">hello world</p>");
        let wrapper2 = ast2.wrapper.borrow();
        let root2 = wrapper2.children[0].borrow();
        assert_eq!(root2.el.class_binding.as_ref().unwrap(), "class1");

        // interpolation warning
        let (_ast3, warnings3) = parse("<p class=\"{{error}}\">hello world</p>");
        assert_eq!(warnings3.borrow().len(), 1);
        assert_eq!(warnings3.borrow()[0], "class=\"{{error}}\": Interpolation inside attributes has been removed. Use v-bind or the colon shorthand instead. For example, instead of <div class=\"{ val }\">, use <div :class=\"val\">.");
    }

    #[test]
    fn style_binding() {
        let (ast, _warnings) = parse("<p :style=\"error\">hello world</p>");
        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.style_binding.as_ref().unwrap(), "error");
    }

    #[test]
    fn attribute_with_v_bind() {
        let (ast, _warnings) = parse("<input type=\"text\" name=\"field1\" :value=\"msg\">");
        let binding = ast.wrapper.borrow();
        let input_ast = binding.children[0].borrow();
        assert_eq!(input_ast.el.attrs[0].name, "type");
        assert_eq!(input_ast.el.attrs[0].value.as_ref().unwrap(), "text");
        assert_eq!(input_ast.el.attrs[1].name, "name");
        assert_eq!(input_ast.el.attrs[1].value.as_ref().unwrap(), "field1");
        assert_eq!(input_ast.el.props[0].name, "value");
        assert_eq!(input_ast.el.props[0].value.as_ref().unwrap(), "msg");
    }

    #[test]
    fn empty_v_bind_expression() {
        let (_ast, warnings) = parse("<div :empty-msg=\"\"></div>");
        assert_eq!(warnings.borrow().len(), 1);
        assert_eq!(
            warnings.borrow()[0],
            "The value for a v-bind expression cannot be empty. Found in \"v-bind:empty-msg\""
        );
    }

    fn parse_v_bind_on(template: &str) -> (ASTTree, Rc<RefCell<Vec<String>>>) {
        let warnings = Rc::new(RefCell::new(Vec::new()));
        let warnings_clone = Rc::clone(&warnings);
        let options = CompilerOptions {
            dev: true,
            is_ssr: false,
            v_bind_prop_short_hand: true,
            preserve_comments: false,
            whitespace_handling: WhitespaceHandling::Condense,
            new_slot_syntax: true,
            is_pre_tag: None,
            get_namespace: None,
            warn: Some(Box::new(move |msg: &str| {
                warnings_clone.borrow_mut().push(msg.to_string());
            })),
            delimiters: None,
            modules: Some(vec![
                Box::new(ClassModule {}),
                Box::new(ModelModule {}),
                Box::new(StyleModule {}),
            ]),
        };
        let mut parser = VueParser::new(&options);

        (parser.parse(template), warnings)
    }

    // v_bind_prop_short_hand == true

    #[test]
    fn v_bind_prop_shorthand_syntax() {
        let (ast, _warnings) = parse_v_bind_on("<div .id=\"foo\"></div>");
        let binding = ast.wrapper.borrow();
        let div_ast = binding.children[0].borrow();
        assert_eq!(div_ast.el.props[0].name, "id");
        assert_eq!(div_ast.el.props[0].value.as_ref().unwrap(), "foo");
    }

    #[test]
    fn v_bind_prop_shorthand_syntax_with_modifiers() {
        let (ast, _warnings) = parse_v_bind_on("<div .id.mod=\"foo\"></div>");
        let binding = ast.wrapper.borrow();
        let div_ast = binding.children[0].borrow();
        assert_eq!(div_ast.el.props[0].name, "id");
        assert_eq!(div_ast.el.props[0].value.as_ref().unwrap(), "foo");
    }

    #[test]
    fn v_bind_prop_shorthand_dynamic_argument() {
        let (ast, _warnings) = parse_v_bind_on("<div .[id]=\"foo\"></div>");
        let binding = ast.wrapper.borrow();
        let div_ast = binding.children[0].borrow();
        assert_eq!(div_ast.el.props[0].name, "id");
        assert_eq!(div_ast.el.props[0].value.as_ref().unwrap(), "foo");
    }

    // TODO: These should give warning but they are not parsed as attributes for now
    // #[test]
    // fn parse_and_warn_invalid_dynamic_arguments() {
    //     let templates = vec![
    //         "<div v-bind:['foo' + bar]=\"baz\"/>",
    //         "<div :['foo' + bar]=\"baz\"/>",
    //         "<div @['foo' + bar]=\"baz\"/>",
    //         "<foo #['foo' + bar]=\"baz\"/>",
    //         "<div :['foo' + bar].some.mod=\"baz\"/>",
    //     ];
    //
    //     for template in templates {
    //         let (ast, warnings) = parse(&template);
    //         assert_eq!(warnings.borrow().len(), 1);
    //         assert_eq!(warnings.borrow()[0], "Invalid dynamic argument expression");
    //     }
    // }

    #[test]
    fn multiple_dynamic_slot_names_without_warning() {
        let (ast, warnings) = parse(
            "<my-component>
            <template #[foo]>foo</template>
            <template #[data]=\"scope\">scope</template>
            <template #[bar]>bar</template>
        </my-component>",
        );

        assert_eq!(warnings.borrow().len(), 0);

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        let root_scoped_slots = root.el.scoped_slots.as_ref().unwrap();

        assert!(root_scoped_slots.contains_key("foo"));
        assert!(root_scoped_slots.contains_key("data"));
        assert!(root_scoped_slots.contains_key("bar"));

        let slot_foo = root_scoped_slots.get("foo").unwrap().borrow();
        let slot_data = root_scoped_slots.get("data").unwrap().borrow();
        let slot_bar = root_scoped_slots.get("bar").unwrap().borrow();

        assert_eq!(slot_foo.el.token.data, Box::from("template"));
        assert_eq!(slot_data.el.token.data, Box::from("template"));
        assert_eq!(slot_bar.el.token.data, Box::from("template"));

        assert_eq!(slot_foo.el.attrs[0].name, "#[foo]");
        assert_eq!(slot_foo.el.attrs[0].value, None);

        assert_eq!(slot_data.el.attrs[0].name, "#[data]");
        assert_eq!(slot_data.el.attrs[0].value, Some("scope".to_string()));

        assert_eq!(slot_bar.el.attrs[0].name, "#[bar]");
        assert_eq!(slot_bar.el.attrs[0].value, None);
    }

    #[test]
    fn special_case_static_attribute_that_must_be_props() {
        let (ast, _warnings) = parse("<video muted></video>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.attrs[0].name, "muted");
        assert_eq!(root.el.attrs[0].value, None);
        assert_eq!(root.el.props[0].name, "muted");
        assert_eq!(root.el.props[0].value, Some("true".to_string()));
    }
}
