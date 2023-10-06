use async_trait::async_trait;
use glimpse::prelude::*;
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
                    let _ = write_clipboard(&*solution.clone());
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

fn fmt_number(mut n: f64) -> Option<String> {
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
        if cat != pre_cat || cat == CharCategory::Paren || cat == CharCategory::Operator {
            if word.len() > 0 {
                append_token(&pre_cat, &mut word, &mut tokens)?;
            }
        }
        pre_cat = cat;
        word.push(c);
    }

    append_token(&pre_cat, &mut word, &mut tokens)?;

    if tokens.len() == 0 {
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

    if ts.len() == 0 {
        Some(n.unwrap())
    } else {
        None
    }
}

type Tokens = Vec<Token>;

fn parse(ts: Tokens) -> Option<(Tokens, Value)> {
    test(ts)
}

fn try_consume(ts: &Tokens, matching: Token) -> Option<Tokens> {
    if ts.iter().next()? == &matching {
        Some(ts[1..].to_vec())
    } else {
        None
    }
}

#[derive(PartialEq)]
enum Value {
    Number(f64),
    Epsilon,
}

impl Value {
    fn wrap(n: f64) -> Value {
        Value::Number(n)
    }

    fn unwrap(self) -> f64 {
        match self {
            Value::Number(n) => n,
            Value::Epsilon => panic!("Fatal calculator parse error. Unwrapped epsilon."),
        }
    }
}

type PartialExpr = Option<(Tokens, Value)>;
type ParserNode = fn(Tokens) -> PartialExpr;
type Operation = fn(f64, f64) -> f64;

fn first(
    l_node: ParserNode,
    r_node: ParserNode,
    operation: Operation,
) -> impl Fn(Tokens) -> PartialExpr {
    move |ts| {
        let (ts, left) = l_node(ts)?;
        let (ts, right) = r_node(ts)?;

        match (left, right) {
            (Value::Number(left), Value::Number(right)) => {
                Some((ts, Value::wrap(operation(left, right))))
            }
            (Value::Number(left), Value::Epsilon) => Some((ts, Value::wrap(left))),
            _ => None,
        }
    }
}

fn follow(first: ParserNode, operator: Token) -> impl Fn(Tokens) -> PartialExpr {
    move |ts| {
        let operator = operator.clone();
        match try_consume(&ts, operator) {
            Some(ts) => first(ts),
            None => Some((ts, Value::Epsilon)),
        }
    }
}

// TODO: Unary minus and plus

fn test(ts: Tokens) -> PartialExpr {
    first(add, assign_, |l, r| if l == r { 1. } else { 0. })(ts)
}

fn assign_(ts: Tokens) -> PartialExpr {
    follow(test, Token::Operator('='))(ts)
}

fn add(ts: Tokens) -> PartialExpr {
    first(sub, add_, |l, r| l + r)(ts)
}

fn add_(ts: Tokens) -> PartialExpr {
    follow(add, Token::Operator('+'))(ts)
}

fn sub(ts: Tokens) -> PartialExpr {
    first(mul, sub_, |l, r| l - r)(ts)
}

fn sub_(ts: Tokens) -> PartialExpr {
    follow(sub, Token::Operator('-'))(ts)
}

fn mul(ts: Tokens) -> PartialExpr {
    first(div, mul_, |l, r| l * r)(ts)
}

fn mul_(ts: Tokens) -> PartialExpr {
    follow(mul, Token::Operator('*'))(ts)
}

fn div(ts: Tokens) -> PartialExpr {
    first(pow, div_, |l, r| l / r)(ts)
}

fn div_(ts: Tokens) -> PartialExpr {
    follow(div, Token::Operator('/'))(ts)
}

fn pow(ts: Tokens) -> PartialExpr {
    first(dice_roll, pow_, |l, r| l.powf(r))(ts)
}

fn pow_(ts: Tokens) -> PartialExpr {
    follow(pow, Token::Operator('^'))(ts)
}

#[rustfmt::skip]
fn call(ts: Tokens) -> PartialExpr {
    let first_token = ts.iter().next()?;
    if let Token::Function(name) = first_token {
        let ts = ts[1..].to_vec();
        let (ts, n) = brack(ts)?;

        let n = n.unwrap(); // brack never returns epsilon
        let n = run_fn(&name, n)?;
        Some((ts, Value::wrap(n)))
    } else {
        brack(ts)
    }
}

fn dice_roll(ts: Tokens) -> PartialExpr {
    let (ts, left) = call(ts)?;
    let (ts, right) = dice_roll_(ts)?;

    match (left, right) {
        (Value::Number(left), Value::Number(right)) => Some((ts, Value::wrap(roll(left, right)?))),
        (Value::Number(left), Value::Epsilon) => Some((ts, Value::wrap(left))),
        _ => None,
    }
}

fn dice_roll_(ts: Tokens) -> PartialExpr {
    let operator = Token::Operator('d');

    match try_consume(&ts, operator) {
        Some(ts) => dice_roll(ts),
        None => Some((ts, Value::Epsilon)),
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

#[rustfmt::skip]
fn brack(ts: Tokens) -> PartialExpr {
    match try_consume(&ts, Token::Paren('(')) {
        Some(ts) => parse(ts)
            .and_then(|(ts, left)|
                match try_consume(&ts, Token::Paren(')')) {
                    Some(ts) => Some((ts, left)),
                    None => None
                }),
        None => unary_minus(ts)
    }
}

fn unary_minus(ts: Tokens) -> PartialExpr {
    let operator = Token::Operator('-');

    match try_consume(&ts, operator) {
        Some(ts) => {
            let (ts, n) = literal(ts)?;
            Some((ts, Value::wrap(-n.unwrap()))) // literal can never return epsilon
        }
        None => literal(ts),
    }
}

fn literal(ts: Tokens) -> PartialExpr {
    if let Some(Token::Number(n)) = ts.iter().next() {
        Some((ts[1..].to_vec(), Value::wrap(*n)))
    } else {
        None
    }
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
