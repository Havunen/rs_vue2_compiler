use rs_html_parser_tokenizer_tokens::QuoteType;

pub struct DirectivesParser {
    len: usize,
    str: String,
    chr: char,
    index: usize,
    expression_pos: usize,
    expression_end_pos: usize,
}

impl DirectivesParser {
    pub fn new(val: &str) -> Self {
        Self {
            len: val.len(),
            str: val.to_string(),
            chr: '\0',
            index: 0,
            expression_pos: 0,
            expression_end_pos: 0,
        }
    }

    pub fn next(&mut self) -> Option<char> {
        self.index += 1;
        if self.index < self.len {
            self.chr = self.str.chars().nth(self.index).unwrap();
            Some(self.chr)
        } else {
            None
        }
    }

    pub fn eof(&self) -> bool {
        self.index >= self.len
    }

    pub fn is_string_start(&self, chr: char) -> bool {
        chr == '\"' || chr == '\''
    }

    pub fn parse_bracket(&mut self) {
        let mut in_bracket = 1;
        self.expression_pos = self.index;
        while !self.eof() {
            self.next();
            if self.is_string_start(self.chr) {
                self.parse_string();
                continue;
            }
            if self.chr == '[' {
                in_bracket += 1;
            }
            if self.chr == ']' {
                in_bracket -= 1;
            }
            if in_bracket == 0 {
                self.expression_end_pos = self.index;
                break;
            }
        }
    }

    pub fn parse_string(&mut self) {
        let string_quote = self.chr;
        while !self.eof() {
            self.next();
            if self.chr == string_quote {
                break;
            }
        }
    }

    pub fn parse(&mut self) -> ModelParseResult {
        while !self.eof() {
            self.next();
            if self.is_string_start(self.chr) {
                self.parse_string();
            } else if self.chr == '[' {
                self.parse_bracket();
            }
        }

        ModelParseResult {
            exp: self.str[0..self.expression_pos].to_string(),
            key: if self.expression_end_pos <= self.expression_pos {None} else { Some(self.str[self.expression_pos + 1..self.expression_end_pos].to_string()) },
        }
    }
}

pub struct ModelParseResult {
    pub exp: String,
    pub key: Option<String>,
}

pub fn parse_model(val: &str) -> ModelParseResult {
    let val = val.trim();
    let mut parser = DirectivesParser::new(val);
    parser.parse()
}

pub fn gen_assignment_code(val: &(Box<str>, QuoteType), assignment: &str) -> String {
    let value = &*val.0;
    let res = parse_model(value);
    match res.key {
        None => format!("{}={}", value, assignment),
        Some(key) => format!("$set({}, {}, {})", res.exp, key, assignment),
    }
}
