use async_trait::async_trait;
use gtk::traits::{ContainerExt, LabelExt, WidgetExt};

use crate::exec::write_clipboard;

use super::{SearchModule, SearchResult};

pub struct Calculator {}

#[async_trait]
impl SearchModule for Calculator {
    async fn search(&self, query: String, _: u32) -> Vec<SearchResult> {
        #[rustfmt::skip]
        let solution = tokenize(&query)
            .and_then(swap_words)
            .and_then(execute)
            .and_then(fmt_number);

        if let Some(solution) = solution {
            let solution_cpy = solution.clone();
            let render = Box::new(move || render(solution_cpy.clone()));

            vec![SearchResult {
                relevance: f32::INFINITY,
                id: 0x3141592653,
                on_select: Some(Box::new(move || {
                    let _ = write_clipboard(&solution.clone());
                })),
                render,
                preview_window_data: crate::preview_window::PreviewWindowShowing::None,
            }]
        } else {
            vec![]
        }
    }
}

fn render(solution: String) -> gtk::Box {
    let word_attributes = pango::AttrList::new();
    let word_desc = pango::FontDescription::from_string("24");
    let word_size_attrib = pango::AttrFontDesc::new(&word_desc);
    word_attributes.insert(word_size_attrib);

    let solution = format!("= {}", solution);
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let label = gtk::Label::new(Some(solution.as_str()));

    label.set_attributes(Some(&word_attributes));
    label.set_halign(gtk::Align::Start);

    container.add(&label);

    let subtext = gtk::Label::new(Some("Press enter to copy to clipboard"));
    let word_attributes = pango::AttrList::new();
    let word_desc = pango::FontDescription::from_string("12");
    let word_size_attrib = pango::AttrFontDesc::new(&word_desc);
    word_attributes.insert(word_size_attrib);

    subtext.set_attributes(Some(&word_attributes));
    subtext.set_halign(gtk::Align::Start);
    subtext.set_opacity(0.5);

    container.add(&subtext);

    container.set_margin(15);
    container
}

fn fmt_number(n: f64) -> Option<String> {
    if n.is_nan() {
        "Math error".to_string().into()
    } else if n.is_infinite() {
        if n.is_sign_positive() {
            "∞".to_string().into()
        } else {
            "-∞".to_string().into()
        }
    } else if n == 0. {
        "0".to_string().into()
    } else {
        sci_notation(n)
    }
}

fn sci_notation(n: f64) -> Option<String> {
    // n = 10^exponent * mantissa
    // log(n) = log(10^exponent * mantissa)
    // log(n) = exponent + log(mantissa)

    // log(mantissa) < 1
    // therefore: floor(log(n)) = exponent

    let exponent = n.abs().log10().floor();

    if exponent.abs() > 3. {
        let mantissa = n / 10_f64.powf(exponent);
        let mantissa = trunc_to_sig_figs(mantissa, 4);

        format!("{}e{}", mantissa, exponent)
    } else {
        format!("{}", n)
    }
    .into()
}

fn trunc_to_sig_figs(n: f64, sig_figs: usize) -> f64 {
    let a = 10_f64.powf((sig_figs - 1) as f64);
    (n * a).round() / a
}

#[derive(PartialEq, Debug, Clone)]
enum Token {
    Number(f64),
    Operator(char),
    Paren(char),
    Word(String),
    Function(String),
}

#[derive(PartialEq)]
enum CharCategory {
    Numeral,
    Operator,
    Paren,
    Letter,
    WhiteSpace,
}

fn catergorise_char(c: &char) -> CharCategory {
    if c.is_numeric() || *c == '.' {
        CharCategory::Numeral
    } else if *c == '+' || *c == '-' || *c == '*' || *c == '/' || *c == '^' || *c == '=' {
        CharCategory::Operator
    } else if *c == '(' || *c == ')' {
        CharCategory::Paren
    } else if c.is_alphabetic() {
        CharCategory::Letter
    } else if c.is_whitespace() {
        CharCategory::WhiteSpace
    } else {
        CharCategory::Letter
    }
}

fn tokenize(exp: &str) -> Option<Vec<Token>> {
    let mut tokens = vec![];

    let mut pre_cat = CharCategory::WhiteSpace;

    let mut word = String::new();

    for c in exp.chars() {
        let cat = catergorise_char(&c);

        // NOTE: Having operator always mark the end of a token only allows single
        //       character operators. If for whatever reason you want to implement
        //       multi-character operators this needs to do something a little smarter.
        if (cat != pre_cat || cat == CharCategory::Paren || cat == CharCategory::Operator)
            && !word.is_empty()
        {
            append_token(&pre_cat, &mut word, &mut tokens)?;
        }
        pre_cat = cat;
        word.push(c);
    }

    append_token(&pre_cat, &mut word, &mut tokens)?;

    if tokens.is_empty() {
        return None;
    }
    Some(tokens)
}

fn append_token(pre_cat: &CharCategory, word: &mut String, tokens: &mut Vec<Token>) -> Option<()> {
    match *pre_cat {
        CharCategory::Numeral => {
            if let Ok(n) = word.parse::<f64>() {
                tokens.push(Token::Number(n));
            }
        }
        CharCategory::Operator => {
            if word.len() != 1 {
                return None;
            }
            tokens.push(Token::Operator(word.chars().next()?));
        }
        CharCategory::Paren => {
            if word.len() != 1 {
                return None;
            }
            tokens.push(Token::Paren(word.chars().next()?));
        }
        CharCategory::Letter => {
            tokens.push(Token::Word(word.clone().to_string()));
        }
        CharCategory::WhiteSpace => {}
    }
    *word = String::new();
    Some(())
}

fn swap_words(tokens: Vec<Token>) -> Option<Vec<Token>> {
    let mut ret = vec![];
    for t in tokens {
        if let Token::Word(s) = t {
            let s = s.to_lowercase();

            const FUNCTIONS: [&str; 10] = [
                "sqrt", "sin", "cos", "tan", "asin", "acos", "atan", "ln", "log", "exp",
            ];

            if s == "plus" {
                ret.push(Token::Operator('+'));
            } else if s == "times" {
                ret.push(Token::Operator('*'));
            } else if s == "div" {
                ret.push(Token::Operator('/'));
            } else if s == "minus" {
                ret.push(Token::Operator('-'));
            } else if s == "pi" {
                ret.push(Token::Number(std::f64::consts::PI));
            } else if s == "d" {
                ret.push(Token::Operator('d'));
            } else if s == "e" {
                ret.push(Token::Number(std::f64::consts::E));
            } else if FUNCTIONS.contains(&s.as_str()) {
                ret.push(Token::Function(s));
            } else {
                return None;
            }
        } else {
            ret.push(t);
        }
    }
    Some(ret)
}

fn execute(tokens: Vec<Token>) -> Option<f64> {
    let (ts, n) = parse(tokens)?;

    if ts.is_empty() {
        Some(n)
    } else {
        None
    }
}

type Tokens = Vec<Token>;
type Value = f64;

fn parse(ts: Tokens) -> Option<(Tokens, Value)> {
    add(ts)
}

fn add(ts: Tokens) -> Option<(Tokens, Value)> {
    let (ts, lhs) = mul(ts)?;

    add_(lhs, ts)
}

fn add_(lhs: Value, ts: Tokens) -> Option<(Tokens, Value)> {
    if let Some(Token::Operator('+')) = peek(&ts) {
        let (ts, rhs) = mul(ts[1..].to_vec())?;

        let this_oper_res = lhs + rhs;

        if let Some(r) = add_(this_oper_res, ts.clone()) {
            return Some(r);
        }

        Some((ts, this_oper_res))
    } else if let Some(Token::Operator('-')) = peek(&ts) {
        let (ts, rhs) = mul(ts[1..].to_vec())?;

        let this_oper_res = lhs - rhs;

        if let Some(r) = add_(this_oper_res, ts.clone()) {
            return Some(r);
        }

        Some((ts, this_oper_res))
    } else {
        Some((ts, lhs))
    }
}

fn mul(ts: Tokens) -> Option<(Tokens, Value)> {
    let (ts, lhs) = unary_minus(ts)?;

    mul_(lhs, ts)
}

fn mul_(lhs: Value, ts: Tokens) -> Option<(Tokens, Value)> {
    if let Some(Token::Operator('*')) = peek(&ts) {
        let (ts, rhs) = unary_minus(ts[1..].to_vec())?;

        let this_oper_res = lhs * rhs;

        if let Some(r) = mul_(this_oper_res, ts.clone()) {
            return Some(r);
        }

        Some((ts, this_oper_res))
    } else if let Some(Token::Operator('/')) = peek(&ts) {
        let (ts, rhs) = unary_minus(ts[1..].to_vec())?;

        let this_oper_res = lhs / rhs;

        if let Some(r) = mul_(this_oper_res, ts.clone()) {
            return Some(r);
        }

        Some((ts, this_oper_res))
    } else {
        Some((ts, lhs))
    }
}

fn unary_minus(ts: Tokens) -> Option<(Tokens, Value)> {
    if let Some(Token::Operator('-')) = peek(&ts) {
        let (ts, n) = pow(ts[1..].to_vec())?;
        Some((ts, -n))
    } else {
        pow(ts)
    }
}

fn pow(ts: Tokens) -> Option<(Tokens, Value)> {
    let (ts, lhs) = brackets(ts)?;

    pow_(lhs, ts)
}

fn pow_(lhs: Value, ts: Tokens) -> Option<(Tokens, Value)> {
    if let Some(Token::Operator('^')) = peek(&ts) {
        let (ts, rhs) = brackets(ts[1..].to_vec())?;

        let this_oper_res = lhs.powf(rhs);

        if let Some(r) = pow_(this_oper_res, ts.clone()) {
            return Some(r);
        }

        Some((ts, this_oper_res))
    } else {
        Some((ts, lhs))
    }
}

fn brackets(ts: Tokens) -> Option<(Tokens, Value)> {
    if let Some(Token::Paren('(')) = peek(&ts) {
        let (ts, n) = parse(ts[1..].to_vec())?;

        try_consume(&ts, Token::Paren(')')).map(|ts| (ts, n))
    } else {
        function(ts)
    }
}

fn function(ts: Tokens) -> Option<(Tokens, Value)> {
    if let Some(Token::Function(name)) = peek(&ts) {
        let (ts, n) = brackets(ts[1..].to_vec())?;

        run_fn(&name, n).map(|n| (ts, n))
    } else {
        dice_roll(ts)
    }
}

fn dice_roll(ts: Tokens) -> Option<(Tokens, Value)> {
    let (ts, lhs) = literal(ts)?;

    if let Some(Token::Operator('d')) = peek(&ts) {
        let (ts, dice_sides) = literal(ts[1..].to_vec())?;

        roll(lhs, dice_sides).map(|n| (ts, n))
    } else {
        Some((ts, lhs))
    }
}

fn literal(ts: Tokens) -> Option<(Tokens, Value)> {
    if let Some(Token::Number(n)) = ts.first() {
        Some((ts[1..].to_vec(), *n))
    } else {
        None
    }
}

fn peek(ts: &Tokens) -> Option<&Token> {
    ts.iter().next()
}

fn try_consume(ts: &Tokens, matching: Token) -> Option<Tokens> {
    if ts.iter().next()? == &matching {
        Some(ts[1..].to_vec())
    } else {
        None
    }
}

fn roll(dice_count: f64, dice_sides: f64) -> Option<f64> {
    if dice_count < 1. || dice_sides < 1. || dice_count.fract() != 0. || dice_sides.fract() != 0. {
        return None;
    }

    let dice_count = dice_count as usize;

    (0..dice_count)
        .map(|_| rand::random::<f64>() * dice_sides)
        .map(|n| n.ceil())
        .sum::<f64>()
        .into()
}

fn run_fn(name: &str, n: f64) -> Option<f64> {
    match name {
        "sqrt" => Some(n.sqrt()),
        "sin" => Some(sin_(n)),
        "cos" => Some(n.cos()),
        "tan" => Some(n.tan()),
        "asin" => Some(n.asin()),
        "acos" => Some(n.acos()),
        "atan" => Some(n.atan()),
        "ln" => Some(n.ln()),
        "log" => Some(n.log10()),
        "exp" => Some(n.exp()),
        _ => None,
    }
}

fn sin_(n: f64) -> f64 {
    if n % (std::f64::consts::PI) == 0. {
        0.
    } else {
        n.sin()
    }
}

impl Calculator {
    pub fn new() -> Calculator {
        Calculator {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math() {
        let exec = |s: &str| {
            tokenize(s)
                .and_then(swap_words)
                .and_then(execute)
                .and_then(fmt_number)
        };

        assert_eq!(exec("2 + 2"), Some("4".to_string()));
        assert_eq!(exec("asdfasdfawsdfuhyk"), None);
        assert_eq!(exec("(15*5 - 6) - 20 "), Some("49".to_string()));
        assert_eq!(exec("15*5 - 6 - 20 "), Some("49".to_string()));
        assert_eq!(exec("-5*5 + 6"), Some("-19".to_string()));
        assert_eq!(exec("-(-15*5 + 6 + 20)"), Some("49".to_string()));
        assert_eq!(exec("-(-15*5 + 6 + 20) ^ 4"), Some("-5.765e6".to_string()));
        assert_eq!(exec("(-15*5 + 6) ^ (sin pi)"), Some("1".to_string()));
    }
}
