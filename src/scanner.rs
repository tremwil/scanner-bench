use crate::pattern::Pattern;

pub trait Scanner {
    fn find_one(haystack: &[u8], pat: &impl Pattern) -> Option<usize>;
    fn find_all(haystack: &[u8], pat: &impl Pattern) -> impl Iterator<Item = usize>;
}
