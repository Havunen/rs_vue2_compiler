use std::borrow::Cow;
use rs_html_parser_tokenizer_tokens::QuoteType;

pub fn parse_filters((expr, quote_type): &(Cow<str>, QuoteType)) -> String {
    let mut in_single = quote_type == &QuoteType::Single;
    let mut in_double = quote_type == &QuoteType::Double;
    let mut in_template_string = false;
    let mut in_regex = false;
    let mut curly = 0;
    let mut square = 0;
    let mut paren = 0;
    let mut last_filter_index = 0;
    let mut c: char = '0';
    let mut prev;
    let mut expression: Option<String> = None;
    let mut filters: Vec<String> = Vec::new();

    for (i, char) in expr.chars().enumerate() {
        prev = c;
        c = char;

        if in_single {
            if c == '\'' && prev != '\\' {
                in_single = false;
            }
        } else if in_double {
            if c == '\"' && prev != '\\' {
                in_double = false;
            }
        } else if in_template_string {
            if c == '`' && prev != '\\' {
                in_template_string = false;
            }
        } else if in_regex {
            if c == '/' && prev != '\\' {
                in_regex = false;
            }
        } else if c == '|' && expr.chars().nth(i + 1).unwrap() != '|' && expr.chars().nth(i - 1).unwrap() != '|' && curly == 0 && square == 0 && paren == 0 {
            if expression.is_none() {
                last_filter_index = i + 1;
                expression = Some(expr[..i].trim().to_string());
            } else {
                push_filter(&mut filters, &expr, &mut last_filter_index, i);
            }
        } else {
            match c {
                '\"' => in_double = true,
                '\'' => in_single = true,
                '`' => in_template_string = true,
                '(' => paren += 1,
                ')' => paren -= 1,
                '[' => square += 1,
                ']' => square -= 1,
                '{' => curly += 1,
                '}' => curly -= 1,
                _ => (),
            }
            if c == '/' {
                let mut j = i - 1;
                let mut p: char = '0';
                while j >= 0 {
                    p = expr.chars().nth(j).unwrap();
                    if p != ' ' {
                        break;
                    }
                    j -= 1;
                }
                if j < 0 || !valid_division_char(p) {
                    in_regex = true;
                }
            }
        }
    }

    if expression.is_none() {
        expression = Some(expr[..].trim().to_string());
    } else if last_filter_index != 0 {
        push_filter(&mut filters, &expr, &mut last_filter_index, expr.len());
    }

    if !filters.is_empty() {
        for filter in filters {
            expression = Some(wrap_filter(expression.unwrap(), filter));
        }
    }

    expression.unwrap()
}

fn push_filter(filters: &mut Vec<String>, exp: &str, last_filter_index: &mut usize, i: usize) {
    filters.push(exp[*last_filter_index..i].trim().to_string());
    *last_filter_index = i + 1;
}

fn valid_division_char(p: char) -> bool {
    p.is_alphanumeric() || p == ')' || p == '.' || p == '+' || p == '-' || p == '_' || p == '$' || p == ']'
}

fn wrap_filter(exp: String, filter: String) -> String {
    let i = filter.find('(');
    if i.is_none() {
        return format!("_f(\"{}\")({})", filter, exp);
    } else {
        let (name, args) = filter.split_at(i.unwrap());
        return format!("_f(\"{}\")({}{})", name, exp, if args != ")" { ",".to_owned() + args } else { args.to_string() });
    }
}
