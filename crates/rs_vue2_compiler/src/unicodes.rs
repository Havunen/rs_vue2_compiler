use unicase::UniCase;
use lazy_static::lazy_static;

pub struct UniCodes {}

lazy_static! {
    pub static ref UC_TYPE: UniCase<&'static str> = UniCase::new("type");
}

lazy_static! {
    pub static ref UC_V_PRE: UniCase<&'static str> = UniCase::new("v-pre");
}
