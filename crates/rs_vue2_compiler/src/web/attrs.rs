use rs_html_parser_tokenizer_tokens::QuoteType;

fn accept_value(tag: &str) -> bool {
    matches!(tag, "input" | "textarea" | "option" | "select" | "progress")
}

pub fn must_use_prop(
    tag: &str,
    type_attribute: &Option<(Box<str>, QuoteType)>,
    name: &str,
) -> bool {
    if name == "value"
        && accept_value(tag)
        && type_attribute
            .as_ref()
            .is_some_and(|x| x.0.as_ref() != "button")
    {
        return true;
    }

    if name == "selected" && tag == "option" {
        return true;
    }

    if name == "checked" && tag == "input" {
        return true;
    }

    if name == "muted" && tag == "video" {
        return true;
    }

    return false;
}
