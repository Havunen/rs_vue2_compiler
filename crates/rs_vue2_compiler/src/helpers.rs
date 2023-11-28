use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use rs_html_parser_tokenizer_tokens::QuoteType;
use unicase::UniCase;

pub fn get_and_remove_attr<'a>(
    el_attrs: &mut Option<BTreeMap<UniCase<&'a str>, Option<(Cow<'a, str>, QuoteType)>>>,
    el_ignored: &mut BTreeSet<UniCase<&'a str>>,
    name: &'a UniCase<&'a str>,
    fully_remove: bool
) -> Option<Cow<'a, str>> {
    if let Some(ref mut attrs) = el_attrs {
        if let Some(attr_value_option) = attrs.get(name) {
            if !fully_remove {
                el_ignored.insert(name.clone());
            }

            if let Some((attr_value, _attr_quote)) = attr_value_option {
                return Some(attr_value.clone());
            }
        }
    }

    return None;
}
