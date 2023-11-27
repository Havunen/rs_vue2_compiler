use unicase::UniCase;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref UC_TYPE: UniCase<&'static str> = UniCase::new("type");
    pub static ref UC_V_PRE: UniCase<&'static str> = UniCase::new("v-pre");
    pub static ref UC_V_FOR: UniCase<&'static str> = UniCase::new("v-for");
}
