use std::mem::swap;

pub mod interface;
pub mod query_parser;
pub mod query_parser_test;
pub mod query_analysis;
pub mod stringpattern;

pub fn steal<T: Default>(x: &mut T) -> T {
    let mut x1 = T::default();
    swap(x, &mut x1);
    return x1;
}
