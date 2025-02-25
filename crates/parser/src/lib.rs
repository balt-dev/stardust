pub mod tokens;
pub mod lexer;
pub mod ast;
pub mod parse;


pub(crate) const STACK_REDZONE: usize = 32 * 1024;
pub(crate) const STACK_SIZE: usize = 1024 * 1024;