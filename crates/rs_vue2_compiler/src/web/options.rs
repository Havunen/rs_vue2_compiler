use crate::web::compiler::class::ClassModule;
use crate::web::compiler::style::StyleModule;
use crate::{CompilerOptions, WhitespaceHandling};

thread_local! {
    static BASE_CONFIG: CompilerOptions = CompilerOptions {
        dev: false,
        is_ssr: false,
        preserve_comments: false,
        whitespace_handling: WhitespaceHandling::Condense,
        is_pre_tag: None,
        get_namespace: None,
        warn: None,
        delimiters: None,
        modules: vec![Box::new(ClassModule {}), Box::new(StyleModule {})]
    };
}
