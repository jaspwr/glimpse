pub trait MonadicOption<T> {
    fn bind<F: FnOnce(T) -> Option<U>, U>(self, f: F) -> Option<U>;
}

impl<T> MonadicOption<T> for Option<T> {
    fn bind<F: FnOnce(T) -> Option<U>, U>(self, f: F) -> Option<U> {
        match self {
            Some(x) => f(x),
            None => None,
        }
    }
}

pub type Relevance = f32;
pub type SearchResultId = u64;

pub trait Trunc {
    fn trunc(self, len: usize) -> Self;
}

impl Trunc for String {
    fn trunc(mut self, len: usize) -> Self {
        if self.len() > len {
            self.truncate(len);
            self.push_str("…");
        }
        self
    }
}
