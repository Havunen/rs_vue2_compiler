#[cfg(test)]
mod tests {
    use rs_vue2_compiler::ast_tree::ASTTree;
    use rs_vue2_compiler::{CompilerOptions, VueParser};
    use std::rc::Rc;

    fn parse(template: &str) -> ASTTree {
        let mut parser = VueParser::new(CompilerOptions {
            dev: true,
            is_ssr: false,
            is_pre_tag: None,
            get_namespace: None,
        });

        parser.parse(template)
    }

    #[test]
    fn simple_element() {
        let ast = parse("<h1>hello world</h1>");

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
        let ast = parse("<h1>{{msg}}</h1>");

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
        let ast = parse("<ul><li>hello world</li></ul>");

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
        let ast = parse("<hr>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("hr"));
        assert_eq!(root.el.plain, true);
        assert_eq!(root.children.len(), 0);
    }

    #[test]
    fn svg_element() {
        let ast = parse("<svg><text>hello world</text></svg>");

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
}
