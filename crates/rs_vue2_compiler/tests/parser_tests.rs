#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use rs_vue2_compiler::ast_tree::ASTTree;
    use rs_vue2_compiler::{CompilerOptions, VueParser, WhitespaceHandling};
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
            }))
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
        let (script_ast, _script_warnings) = parse("<script type=\"text/javascript\">alert(\"hello world!\")</script>");

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
        assert_eq!(warnings.borrow()[0], "Component template requires a root element, rather than just text.");
    }
    /*

    Convert these when its known how to get provide warnings

      it('not contain root element', () => {
        parse('hello world', baseOptions)
        expect(
          'Component template requires a root element, rather than just text'
        ).toHaveBeenWarned()
      })

      it('warn text before root element', () => {
        parse('before root {{ interpolation }}<div></div>', baseOptions)
        expect(
          'text "before root {{ interpolation }}" outside root element will be ignored.'
        ).toHaveBeenWarned()
      })

      it('warn text after root element', () => {
        parse('<div></div>after root {{ interpolation }}', baseOptions)
        expect(
          'text "after root {{ interpolation }}" outside root element will be ignored.'
        ).toHaveBeenWarned()
      })

      it('warn multiple root elements', () => {
        parse('<div></div><div></div>', baseOptions)
        expect(
          'Component template should contain exactly one root element'
        ).toHaveBeenWarned()
      })
     */

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
        let (script_ast, _script_warnings) = parse("<script type=\"text/javascript\">alert(\"hello world!\")</script>");

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

    /*

        it('not contain root element', () => {
      parse('hello world', baseOptions)
      expect(
        'Component template requires a root element, rather than just text'
      ).toHaveBeenWarned()
    })

    it('warn text before root element', () => {
      parse('before root {{ interpolation }}<div></div>', baseOptions)
      expect(
        'text "before root {{ interpolation }}" outside root element will be ignored.'
      ).toHaveBeenWarned()
    })

    it('warn text after root element', () => {
      parse('<div></div>after root {{ interpolation }}', baseOptions)
      expect(
        'text "after root {{ interpolation }}" outside root element will be ignored.'
      ).toHaveBeenWarned()
    })

    it('warn multiple root elements', () => {
      parse('<div></div><div></div>', baseOptions)
      expect(
        'Component template should contain exactly one root element'
      ).toHaveBeenWarned()
    })

       */

    #[test]
    fn remove_text_nodes_between_v_if_conditions() {
        let (ast, _warnings) = parse("<div><foo v-if=\"1\"></foo> <section v-else-if=\"2\"></section> <article v-else></article> <span></span></div>");

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
    }
}
