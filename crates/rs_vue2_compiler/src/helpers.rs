use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use rs_html_parser_tokenizer_tokens::QuoteType;
use unicase::UniCase;
use crate::ast_elements::ASTElement;
use crate::filter_parser::parse_filters;

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

pub fn get_and_remove_attr_including_quotes<'a>(
    el_attrs: &'a mut Option<BTreeMap<UniCase<&'a str>, Option<(Cow<'a, str>, QuoteType)>>>,
    el_ignored: &'a mut BTreeSet<UniCase<&'a str>>,
    name: &'a UniCase<&'a str>,
    fully_remove: bool
) -> Option<(Cow<'a, str>, QuoteType)> {
    if let Some(ref mut attrs) = el_attrs {
        if let Some(attr_value_option) = attrs.get(name) {
            if !fully_remove {
                el_ignored.insert(name.clone());
            }

            return (attr_value_option).clone();
        }
    }

    return None;
}

pub fn get_binding_attr<'a>(
    el_attrs: &'a mut Option<BTreeMap<UniCase<&'a str>, Option<(Cow<'a, str>, QuoteType)>>>,
    el_ignored: &'a mut BTreeSet<UniCase<&'a str>>,
    name: &'a UniCase<&'a str>,
    get_static: bool
) -> String  {
    let semicolon_val = ":".to_owned() + &*(**name).to_string();
    let key = UniCase::new(semicolon_val.as_str());

    let mut dynamic_value = get_and_remove_attr_including_quotes(el_attrs, el_ignored, &key, false);
    // if dynamic_value.is_none() {
    //     // let semicolon_val = "v-bind:".to_owned() + &*(**name).to_string();
    //     // let key = UniCase::new(semicolon_val.as_str());
    //     //
    //     // dynamic_value = get_and_remove_attr_including_quotes(el_attrs, el_ignored, &key, false);
    // }

    if let Some(found_dynamic_value) = dynamic_value {
        return parse_filters(&found_dynamic_value)
    }

    return String::from("")
}
