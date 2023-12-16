use std::cell::RefCell;
use std::rc::{Rc, Weak};
use crate::ast_elements::{ASTElement};

#[derive(Debug)]
pub struct ASTNode<'a> {
    pub element: ASTElement,
    pub children: RefCell<Vec<Rc<ASTNode<'a>>>>,
    parent: RefCell<Weak<ASTNode<'a>>>,
}

#[derive(Debug)]
pub struct ASTTree<'a> {
    pub root: Option<ASTNode<'a>>,
}

impl <'a> ASTTree<'a> {
    pub fn new(element: ASTElement) -> Self {
        ASTTree {
            root: Some(ASTNode {
                element,
                children: RefCell::new(vec![]),
                parent: RefCell::new(Default::default()),
            })
        }
    }

    pub fn create(&'a self, element: ASTElement, children: RefCell<Vec<Rc<ASTNode<'a>>>>) -> Rc<ASTNode<'a>> {
        let rc = Rc::new(ASTNode {
            element,
            children,
            parent: RefCell::new(Default::default())
        });

        for ch in rc.children.borrow().iter() {
            *ch.parent.borrow_mut() = Rc::downgrade(&rc);
        }
        return rc;
    }

    pub fn append(&'a self, parent: &Rc<ASTNode<'a>>, child: &Rc<ASTNode<'a>>) {
        parent.children.borrow_mut().push(Rc::clone(child));
        *child.parent.borrow_mut() = Rc::downgrade(parent);
    }
}
