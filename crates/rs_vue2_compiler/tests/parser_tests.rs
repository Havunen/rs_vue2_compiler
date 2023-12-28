#[cfg(test)]
mod tests {
    use rs_vue2_compiler::{CompilerOptions, VueParser};

    #[test]
    fn simple_element() {
        let mut parser = VueParser::new(CompilerOptions {
            dev: true,
            is_ssr: false,
            is_pre_tag: None,
        });
        let ast = parser.parse("<h1>hello world</h1>");

        let wrapper = ast.wrapper.borrow();
        let root = wrapper.children[0].borrow();
        assert_eq!(root.el.token.data, Box::from("h1"));
        assert_eq!(root.el.plain, true);
        assert_eq!(root.children[0].borrow().el.token.data, Box::from("hello world"));
    }
}
