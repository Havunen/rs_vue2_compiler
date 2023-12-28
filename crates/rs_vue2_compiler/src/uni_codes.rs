use lazy_static::lazy_static;
use unicase::UniCase;

lazy_static! {
    pub static ref UC_TYPE: UniCase<&'static str> = UniCase::new("type");
    pub static ref UC_V_PRE: UniCase<&'static str> = UniCase::new("v-pre");
    pub static ref UC_V_FOR: UniCase<&'static str> = UniCase::new("v-for");
    pub static ref UC_V_IF: UniCase<&'static str> = UniCase::new("v-if");
    pub static ref UC_V_ELSE: UniCase<&'static str> = UniCase::new("v-else");
    pub static ref UC_V_ELSE_IF: UniCase<&'static str> = UniCase::new("v-else-if");
    pub static ref UC_V_ONCE: UniCase<&'static str> = UniCase::new("v-once");
    pub static ref UC_KEY: UniCase<&'static str> = UniCase::new("key");
}
