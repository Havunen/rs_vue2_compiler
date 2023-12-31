pub trait WarnLogger {
    fn call(&mut self, msg: &str);
    fn clone_box(&self) -> Box<dyn WarnLogger>;
}

pub trait CloneableWarnLogger: WarnLogger {
    fn clone_box(&self) -> Box<dyn CloneableWarnLogger>;
}

impl<T> CloneableWarnLogger for T
where
    T: WarnLogger + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn CloneableWarnLogger> {
        Box::new(self.clone())
    }
}

impl<F> WarnLogger for F
where
    F: FnMut(&str) + Clone + 'static,
{
    fn call(&mut self, msg: &str) {
        self(msg)
    }

    fn clone_box(&self) -> Box<dyn WarnLogger> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn WarnLogger> {
    fn clone(&self) -> Box<dyn WarnLogger> {
        self.clone_box()
    }
}
