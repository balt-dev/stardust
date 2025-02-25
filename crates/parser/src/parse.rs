use either::Either;

use crate::{lexer::Lexer, tokens::{Token, TokenType}, ast::*};


macro_rules! call {
    ($name: expr, $($args: expr),*) => {
        stacker::maybe_grow(crate::STACK_REDZONE, crate::STACK_SIZE, || $name($($args),*))
    };
    (? $func: expr) => {
        match $func {
            Some(v) => v,
            None => return Ok(None)
        }
    };
    (? $name: expr, $($args: expr),*) => {
        match stacker::maybe_grow(crate::STACK_REDZONE, crate::STACK_SIZE, || $name($($args),*)) {
            Some(v) => v,
            None => return Ok(None)
        }
    }
}

macro_rules! match_tok {
    ($expr: expr, $(($tt: tt $(@ $vn: ident)? => $arm: expr)),* $(,)?) => {{
        let v = $expr;
        match v {
            $(Token![$tt #($($vn)?)] => $arm,)*
            _ => return Err(ParseError::TokenMismatch {
                expected: vec![$(Token![$tt #]),*],
                found: v
            })
        }
    }};
}

macro_rules! expect_tok {
    ($tt: tt $expr: expr) => {
        match_tok!($expr, ($tt @ s => s))
    };
}

macro_rules! get_child {
    ($parent: expr, $lexer: expr) => {{
        let Some(child) = $lexer.active_child.clone() else {
            return Err(ParseError::NoChild { parent: $parent.into() })
        };
        *child
    }}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    TokenMismatch {
        expected: Vec<TokenType>,
        found: Token
    },
    NoChild { parent: Token }
}

pub type ParseResult<T> = Result<T, ParseError>;

pub fn parse_module(lexer: &mut Lexer) -> ParseResult<Vec<Item>> {
    let mut items = vec![];
    while let Some(item) = call!(parse_item, lexer)? {
        items.push(item)
    }
    Ok(items)
}

fn parse_item(lexer: &mut Lexer) -> ParseResult<Option<Item>> {
    let vis = call!(? parse_vis, lexer);
    Ok(Some(match_tok! {lexer.next(),
        (import @ import_kw => {
            let path = call!(parse_import_path, lexer)?;
            let semicolon = expect_tok!(; lexer.next());
            Item::Import(Import {
                vis, import_kw, path, semicolon 
            })
        }),
        (external @ external_kw => {
            let module_kw = expect_tok!(module lexer.next());
            let name = expect_tok!(Identifier lexer.next());
            let brace = expect_tok!({} lexer.next());
            let mut child = get_child!(brace, lexer);
            let items = call!(parse_external_module, &mut child)?;
            Item::ExternalModule(ExternalModule {
                vis, external_kw, module_kw, name, brace, items
            })
        }),
        (module @ module_kw => {
            let name = expect_tok!(Identifier lexer.next());
            let brace = expect_tok!({} lexer.next());
            let mut child = get_child!(brace, lexer);
            let items = call!(parse_module, &mut child)?;
            Item::Module(Module {
                vis, module_kw, name, brace, items
            })
        }),
        (constant @ constant_kw => {
            let decl = call!(parse_variable_decl, lexer)?;
            let semicolon = expect_tok!(; lexer.next());
            Item::Constant(Constant {
                vis, constant_kw, decl, semicolon
            })
        }),
        (static @ static_kw => {
            let decl = call!(parse_variable_decl, lexer)?;
            let semicolon = expect_tok!(; lexer.next());
            Item::Static(Static {
                vis, static_kw, decl, semicolon
            })
        }), 
        (global @ global_kw => {
            let decl = call!(parse_variable_decl, lexer)?;
            let semicolon = expect_tok!(; lexer.next());
            Item::Global(Global {
                vis, global_kw, decl, semicolon
            })
        }),
        (struct => todo!()),
        (union => todo!()),
        (enum => todo!()),
        (function => todo!()),
    }))
}

fn parse_vis(lexer: &mut Lexer) -> Option<Visibility> {
    let pre = lexer.clone();
    Some(match lexer.next() {
        Token::Public(p) => Visibility::Public(p),
        Token::Internal(i) => Visibility::Internal(i),
        _ => {
            *lexer = pre;
            Visibility::Private
        }
    })
}

fn parse_import_path(lexer: &mut Lexer) -> ParseResult<ImportPath> {
    todo!()
}

pub fn parse_external_module(lexer: &mut Lexer) -> ParseResult<Vec<ExternItem>> {
    let mut items = vec![];
    while let Some(item) = call!(parse_external_item, lexer)? {
        items.push(item)
    }
    Ok(items)
}

fn parse_external_item(lexer: &mut Lexer) -> ParseResult<Option<ExternItem>> {
    Ok(Some(match_tok! {lexer.next(),
        (global @ global_kw => {
            let argument = call!(parse_argument, lexer)?;
            let semicolon = expect_tok!(; lexer.next());
            ExternItem::ExtGlobal(ExtGlobal {
                global_kw, argument, semicolon
            })
        }),
        (type @ type_kw => {
            let name = expect_tok!(Identifier lexer.next());
            let semicolon = expect_tok!(; lexer.next());
            ExternItem::ExtType(ExtType {
                type_kw, name, semicolon
            })
        }),
        (function @ function_kw => {
            let header = call!(parse_function_header, lexer)?;
            let semicolon = expect_tok!(; lexer.next());
            ExternItem::ExtFunction(ExtFunction {
                function_kw, header, semicolon
            })
        }),
    }))
}

fn parse_function_header(lexer: &mut Lexer) -> ParseResult<FuncHeader> {
    let parens = expect_tok!(() lexer.next());
    let mut child = get_child!(parens, lexer);
    let mut arguments = Separated::<_, Token![,]>::default();
    loop {
        let prefix = child.next();
        let prefix = match_tok! {prefix,
            (let @ let_kw => Either::Left(let_kw)),
            (with @ with_kw => Either::Right(with_kw)),
            (EOB => break)
        };
        let arg = call!(parse_argument, lexer)?;
        let comma = match child.next() {
            Token::EndOfBlock(_) => {
                arguments.end = Some(Box::new((prefix, arg)));
                break;
            },
            other => expect_tok!(, other)
        };
        arguments.parts.push(((prefix, arg), comma))
    }
    let mut prearrow = lexer.clone();
    let mut return_ty = None;
    if let Token::Arrow(arrow) = prearrow.next() {
        *lexer = prearrow;
        let ty = call!(parse_type, lexer)?;
        return_ty = Some((arrow, ty));
    }
    Ok(FuncHeader { parens, arguments, return_ty })
}

fn parse_type(lexer: &mut Lexer) -> ParseResult<Type> {
    let pre = lexer.clone();
    let mut ty = match_tok!{ lexer.next(), 
        (Identifier => {
            *lexer = pre;
            let path = call!(parse_path, lexer)?;
            Type::Path(path)
        }),
        (* @ asterisk => {
            let pre = lexer.clone();
            let constant_kw = match lexer.next() {
                Token![constant #(con)] => Some(con),
                _ => { *lexer = pre; None }
            };
            let target = Box::new(call!(parse_type, lexer)?);
            Type::PointerTy(PointerTy { asterisk, constant_kw, target })
        }),
        (! @ tok => { Type::Never(tok) }),
        (function @ function_kw => {
            let parens = expect_tok!(() lexer.next());
            let mut child = get_child!(parens, lexer);
            let mut arguments = Separated::default();
            loop {
                let ty = call!(parse_type, &mut child)?;
                let comma = match child.next() {
                    Token::EndOfBlock(_) => {
                        arguments.end = Some(Box::new(ty));
                        break;
                    },
                    other => expect_tok!(, other)
                };
                arguments.parts.push((ty, comma))
            }
            let return_ty = if let Token![-> #(arrow)] = lexer.lookahead() {
                lexer.next();
                Some((arrow, Box::new(call!(parse_type, lexer)?)))
            } else { None };
            Type::FunctionTy(FunctionTy {
                function_kw, parens, arguments, return_ty
            })
        }),
        (() @ paren => {
            let mut inner = get_child!(paren, lexer);
            call!(parse_type, &mut inner)?
        })
    };

    let mut pre = lexer.clone();
    while let Token![[] #(brackets)] = lexer.next() {
        let mut inner = get_child!(brackets, lexer);
        let length = (!matches!(inner.lookahead(), Token::EndOfBlock(_)))
            .then(|| call!(parse_expression, &mut inner).map(Box::new)).transpose()?;
        pre = lexer.clone();
        ty = Type::ArrayTy(ArrayTy {
            target: Box::new(ty), brackets, length
        })
    }
    *lexer = pre;

    Ok(ty)
}

fn parse_path(lexer: &mut Lexer) -> ParseResult<Path> {
    let mut i = expect_tok!(Identifier lexer.next());
    let mut sep = Separated::default();
    loop {
        let pre = lexer.clone();
        let dcol = match lexer.next() {
            Token![:: #(dc)] => dc,
            _ => { *lexer = pre; break }
        };
        sep.parts.push((i, dcol));
        i = expect_tok!(Identifier lexer.next());
    }
    sep.end = Some(Box::new(i));
    Ok(sep)
}

fn parse_variable_decl(lexer: &mut Lexer) -> ParseResult<VariableDeclaration> {
    let argument = call!(parse_argument, lexer)?;
    let equal = expect_tok!(= lexer.next());
    let expr = call!(parse_expression, lexer)?;
    let semicolon = expect_tok!(; lexer.next());
    Ok(VariableDeclaration {
        argument, equal, expr, semicolon
    })
}

fn parse_argument(lexer: &mut Lexer) -> ParseResult<Argument> {
    let name = expect_tok!(Identifier lexer.next());
    let colon = expect_tok!(: lexer.next());
    let ty = call!(parse_type, lexer)?;
    Ok(Argument { name, colon, ty })
}

fn parse_expression(lexer: &mut Lexer) -> ParseResult<Expression> {
    todo!()
}