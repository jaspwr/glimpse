pub type Relevance = f32;
pub type SearchResultId = u64;

pub fn cmps<A, B, C, G, F>(g: G, f: F) -> impl Fn(A) -> C
where
    F: Fn(A) -> B,
    G: Fn(B) -> C,
{
    move |x| g(f(x))
}

pub trait Trunc {
    fn trunc(self, len: usize) -> Self;
}

impl Trunc for String {
    fn trunc(mut self, len: usize) -> Self {
        if self.len() > len {
            self.truncate(len);
            self.push('…');
        }
        self
    }
}
