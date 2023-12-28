pub fn to_camel(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = false;
    for c in s.chars() {
        if c == ' ' || c == '_' || c == '-' {
            capitalize_next = true;
        } else {
            if capitalize_next {
                result.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(c.to_ascii_lowercase());
            }
        }
    }
    result
}

/**
 * Hyphenate a camelCase string.
 */
pub fn to_hyphen_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if i != 0 {
                result.push('-');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

pub fn is_some_and_ref<T>(item: &Option<T>, f: impl FnOnce(&T) -> bool) -> bool {
    match item {
        None => false,
        Some(x) => f(&x),
    }
}
