use async_trait::async_trait;
use gtk::traits::{ContainerExt, LabelExt, WidgetExt};

use super::{SearchModule, SearchResult};

use glimpse::prelude::*;

pub struct Calculator {}

#[async_trait]
impl SearchModule for Calculator {
    fn is_ready(&self) -> bool {
        true
    }

    async fn search(&self, query: String, _: u32) -> Vec<SearchResult> {
        #[rustfmt::skip]
        let solution = tokenize(&query)
            .bind(swap_words)
            .bind(execute)
            .bind(fmt_number);

        if let Some(solution) = solution {
            let render = Box::new(move || render(solution.clone()));

            vec![SearchResult {
                relevance: 20.0,
                id: 0xab1489fd,
                on_select: None,
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
    container.set_margin(15);
    container
}

fn fmt_number(n: f64) -> Option<String> {
    if n.is_nan() {
        Some("Math error".to_string())
    } else if n.is_infinite() {
        if n.is_sign_positive() {
            Some("∞".to_string())
        } else {
            Some("-∞".to_string())
        }
    } else {
        // TODO: Scientific notation
        Some(format!("{}", n))
    }
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
    } else if *c == '+' || *c == '-' || *c == '*' || *c == '/' || *c == '^' {
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
        if cat != pre_cat || cat == CharCategory::Paren {
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
        Some(n)
    } else {
        None
    }
}

type Tokens = Vec<Token>;

fn parse(ts: Tokens) -> Option<(Tokens, f64)> {
    add(ts)
}

fn try_consume(ts: &Tokens, matching: Token) -> Option<Tokens> {
    if ts.iter().next()? == &matching {
        Some(ts[1..].to_vec())
    } else {
        None
    }
}

type PartialExpr = Option<(Tokens, f64)>;
type ParserNode = fn(Tokens) -> PartialExpr;
type ParserNodeClosure = Box<dyn Fn(Tokens) -> PartialExpr>;
type Operation = fn(f64, f64) -> f64;

fn first(l_node: ParserNode, r_node: ParserNode, operation: Operation) -> ParserNodeClosure {
    Box::new(move |ts| {
        let (ts, left) = l_node(ts)?;
        let (ts, right) = r_node(ts)?;
        Some((ts, operation(left, right)))
    })
}

fn follow(first: ParserNode, operator: Token, identity: f64) -> ParserNodeClosure {
    Box::new(move |ts| {
        let operator = operator.clone();
        match try_consume(&ts, operator) {
            Some(ts) => first(ts),
            None => Some((ts, identity)),
        }
    })
}

// TODO: Unary minus and plus
fn add(ts: Tokens) -> PartialExpr {
    first(sub, add_, |l, r| l + r)(ts)
}

fn add_(ts: Tokens) -> PartialExpr {
    follow(add, Token::Operator('+'), 0.0)(ts)
}

fn sub(ts: Tokens) -> PartialExpr {
    first(mul, sub_, |l, r| l - r)(ts)
}

fn sub_(ts: Tokens) -> PartialExpr {
    follow(sub, Token::Operator('-'), 0.0)(ts)
}

fn mul(ts: Tokens) -> PartialExpr {
    first(div, mul_, |l, r| l * r)(ts)
}

fn mul_(ts: Tokens) -> PartialExpr {
    follow(mul, Token::Operator('*'), 1.0)(ts)
}

fn div(ts: Tokens) -> PartialExpr {
    first(pow, div_, |l, r| l / r)(ts)
}

fn div_(ts: Tokens) -> PartialExpr {
    follow(div, Token::Operator('/'), 1.0)(ts)
}

fn pow(ts: Tokens) -> PartialExpr {
    first(dice_roll, pow_, |l, r| l.powf(r))(ts)
}

fn pow_(ts: Tokens) -> PartialExpr {
    follow(pow, Token::Operator('^'), 1.0)(ts)
}

#[rustfmt::skip]
fn call(ts: Tokens) -> Option<(Tokens, f64)> {
    let first_token = ts.iter().next()?;
    if let Token::Function(name) = first_token {
        let ts = ts[1..].to_vec();
        let (ts, n) = brack(ts)?;
        let n = run_fn(&name, n)?;
        Some((ts, n))
    } else {
        brack(ts)
    }
}

fn dice_roll(ts: Tokens) -> PartialExpr {
    let (ts, left) = call(ts)?;
    let (ts, right) = dice_roll_(ts)?;

    if right == -1. {
        return Some((ts, left));
    }

    Some((ts, roll(left, right)?))
}

fn dice_roll_(ts: Tokens) -> PartialExpr {
    let operator = Token::Operator('d');

    match try_consume(&ts, operator) {
        Some(ts) => dice_roll(ts),
        None => Some((ts, -1.)),
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
fn brack(ts: Tokens) -> Option<(Tokens, f64)> {
    match try_consume(&ts, Token::Paren('(')) {
        Some(ts) => parse(ts)
            .bind(|(ts, left)|
                match try_consume(&ts, Token::Paren(')')) {
                    Some(ts) => Some((ts, left)),
                    None => None
                }),
        None => literal(ts)
    }
}

fn literal(ts: Tokens) -> Option<(Tokens, f64)> {
    if let Some(Token::Number(n)) = ts.iter().next() {
        Some((ts[1..].to_vec(), *n))
    } else {
        None
    }
}

fn run_fn(name: &str, n: f64) -> Option<f64> {
    match name {
        "sqrt" => Some(n.sqrt()),
        "sin" => Some(n.sin()),
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

impl Calculator {
    pub fn new() -> Calculator {
        Calculator {}
    }
}
