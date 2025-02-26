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
    ($expr: expr, $([default $(@ $vnd: ident)? => $armd: expr],)? $(($tt: tt $([$($tts: tt)*])? $(@ $vn: ident)? => $arm: expr), )*) => {{
        let v = $expr;
        match v {
            $(Token![$tt $($($tts)*)? #($($vn)?)] => $arm,)*
            $(_ $(@ $vnd)? => $armd,)?
            #[allow(unreachable_patterns)]
            _ => return Err(ParseError::TokenMismatch {
                expected: vec![$(Token![$tt $($($tts)*)? #]),*],
                found: v
            })
        }
    }};
}

macro_rules! expect_tok {
    ($tt: tt $expr: expr) => {
        match_tok!($expr, ($tt @ s => s),)
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
    let vis = call!(parse_vis, lexer);
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
        (struct @ struct_kw => {
            let name = expect_tok!(Identifier lexer.next());
            let brace = expect_tok!({} lexer.next());
            let mut child = get_child!(brace, lexer);
            let fields = parse_sep(
                &mut child,
                parse_field,
                |lex| Ok(expect_tok!(, lex.next())),
                true
            )?;
            Item::Struct(Struct{
                vis, struct_kw, name, brace, fields
            })
        }),
        (union @ union_kw => {
            let name = expect_tok!(Identifier lexer.next());
            let brace = expect_tok!({} lexer.next());
            let mut child = get_child!(brace, lexer);
            let fields = parse_sep(
                &mut child,
                parse_field,
                |lex| Ok(expect_tok!(, lex.next())),
                true
            )?;
            Item::Union(Union{
                vis, union_kw, name, brace, fields
            })
        }),
        (enum @ enum_kw => {
            let name = expect_tok!(Identifier lexer.next());
            let colon = expect_tok!(: lexer.next());
            let repr = parse_type(lexer)?;
            let brace = expect_tok!({} lexer.next());
            let mut child = get_child!(brace, lexer);
            let variants = parse_sep(
                &mut child,
                parse_variant,
                |lex| Ok(expect_tok!(, lex.next())),
                true
            )?;
            Item::Enum(Enum{
                vis, enum_kw, name, colon, repr, brace, variants
            })
        }),
        (function @ function_kw => {
            todo!()
        }),
        (EOB => { return Ok(None) }),
    }))
}

fn parse_vis(lexer: &mut Lexer) -> Visibility {
    let pre = lexer.clone();
    match lexer.next() {
        Token::Public(p) => Visibility::Public(p),
        Token::Internal(i) => Visibility::Internal(i),
        _ => {
            *lexer = pre;
            Visibility::Private
        }
    }
}

fn parse_import_path(lexer: &mut Lexer) -> ParseResult<ImportPath> {
    Ok(match_tok!(lexer.next(),
        ({} @ brace => {
            let mut child = get_child!(brace, lexer);
            let parts = parse_sep(
                &mut child,
                parse_import_path,
                |lex| Ok(expect_tok!(, lex.next())),
                true
            )?;
            ImportPath::ModFork(ModFork {
                brace, parts
            })
        }),
        (parent @ parent_kw => {
            let double_colon = expect_tok!(:: lexer.next());
            let next = Box::new(call!(parse_import_path, lexer)?);
            ImportPath::ModParent(ModParent{
                parent_kw, double_colon, next
            })
        }),
        (Identifier @ name => {
            if let Token![:: #(double_colon)] = lexer.lookahead() {
                lexer.next();
                let next = Box::new(call!(parse_import_path, lexer)?);
                ImportPath::ModPath(ModPath{
                    name, double_colon, next
                })
            } else {
                ImportPath::ModEnd(name)
            }
        }),
    ))
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
        (EOB => { return Ok(None) }),
    }))
}

fn parse_function_header(lexer: &mut Lexer) -> ParseResult<FuncHeader> {
    let parens = expect_tok!(() lexer.next());
    let mut child = get_child!(parens, lexer);
    let arguments = parse_sep(
        &mut child,
        |lex| {
            let prefix = match_tok! {lex.next(),
                (let @ let_kw => Either::Left(let_kw)),
                (with @ with_kw => Either::Right(with_kw)),
            };
            let arg = call!(parse_argument, lex)?;
            Ok((prefix, arg))
        },
        |lex| Ok(expect_tok!(, lex.next())),
        true
    )?;
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
        (* [.] @ asterisk_dot => {
            let pre = lexer.clone();
            let constant_kw = match lexer.next() {
                Token![constant #(con)] => Some(con),
                _ => { *lexer = pre; None }
            };
            let target = Box::new(call!(parse_type, lexer)?);
            Type::NonNullPointerTy(NonNullPointerTy { asterisk_dot, constant_kw, target })
        }),
        (any @ tok => { Type::Arbitrary(tok) }),
        (function @ function_kw => {
            let parens = expect_tok!(() lexer.next());
            let mut child = get_child!(parens, lexer);
            let arguments = parse_sep(
                &mut child,
                parse_type,
                |lex| Ok(expect_tok!(, lex.next())),
                true
            )?;
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
        }),
    };

    let mut pre = lexer.clone();
    loop {
        match lexer.next() {
            Token![? #(question)] =>
                ty = Type::OptionalTy(OptionalTy {
                    target: Box::new(ty), question
                }),
            Token![[] #(brackets)] => {
                let mut inner = get_child!(brackets, lexer);
                let length = (!matches!(inner.lookahead(), Token::EndOfBlock(_)))
                    .then(|| call!(parse_expression, &mut inner).map(Box::new)).transpose()?;
                pre = lexer.clone();
                ty = Type::ArrayTy(ArrayTy {
                    target: Box::new(ty), brackets, length
                })
            },
            _ => break
        }
    }
    *lexer = pre;

    Ok(ty)
}

fn parse_sep<T, S>(lexer: &mut Lexer, el: fn(&mut Lexer) -> ParseResult<T>, sep: fn(&mut Lexer) -> ParseResult<S>, allow_trailing: bool) -> ParseResult<Separated<T, S>> {
    let mut arguments = Separated::default();
    loop {
        if allow_trailing {
            if let Token![EOB #()] = lexer.lookahead() { break; }
        }
        let expr = call!(el, lexer)?;
        let comma = match lexer.lookahead() {
            Token::EndOfBlock(_) => {
                arguments.end = Some(Box::new(expr));
                break;
            },
            _ => call!(sep, lexer)?
        };
        arguments.parts.push((expr, comma))
    }
    Ok(arguments)
}

#[inline] fn parse_tuple(lexer: &mut Lexer) -> ParseResult<Separated<Expression, Token![,]>> {
    parse_sep(lexer, parse_expression, |lex| Ok(expect_tok!(, lex.next())), true)
}

#[inline] fn parse_path(lexer: &mut Lexer) -> ParseResult<Path> {
    parse_sep(
        lexer, 
        |lex| Ok(expect_tok!(Identifier lex.next())),
        |lex| Ok(expect_tok!(:: lex.next())),
        false
    )
}

fn parse_field(lexer: &mut Lexer) -> ParseResult<Field> {
    let vis = call!(parse_vis, lexer);
    let argument = call!(parse_argument, lexer)?;
    Ok(Field{vis, argument})
}

fn parse_variant(lexer: &mut Lexer) -> ParseResult<Variant> {
    let name = expect_tok!(Identifier lexer.next());
    let value = matches!(lexer.lookahead(), Token![= #()]).then(|| {
        let eq = expect_tok!(= lexer.next());
        let expr = call!(parse_expression, lexer)?;
        Ok((eq, expr))
    }).transpose()?;
    Ok(Variant { name, value })
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
    let ty = if let Token![: #(colon)] = lexer.lookahead() {
        lexer.next();
        let ty = call!(parse_type, lexer)?;
        Some((colon, ty))
    } else { None };
    Ok(Argument { name, ty })
}

fn parse_atom(lexer: &mut Lexer) -> ParseResult<Atom> {
    match_tok!(lexer.lookahead(),
        [default => call!(parse_literal, lexer).map(Atom::Literal)],
        (Identifier => call!(parse_path, lexer).map(Atom::Path)),
        (size @ size_kw => {
            lexer.next();
            call!(parse_type, lexer).map(|ty| Atom::Size(Size { size_kw, ty }))
        }),
        (alignment @ alignment_kw => {
            lexer.next();
            call!(parse_type, lexer).map(|ty| Atom::Alignment(Alignment { alignment_kw, ty }))
        }),
        (allocate @ allocate_kw => {
            lexer.next();
            if let Token![[] #(brackets)] = lexer.lookahead() {
                lexer.next();
                let mut child = get_child!(brackets, lexer);
                let length = call!(parse_expression, &mut child).map(Box::new)?;
                expect_tok!(EOB child.next());
                let ty = call!(parse_type, lexer)?;
                Ok(Atom::AllocateArray(AllocateArray {
                    allocate_kw, brackets, length, ty
                }))
            } else {
                let ty = call!(parse_type, lexer)?;
                Ok(Atom::Allocate(Allocate {
                    allocate_kw, ty
                }))
            }
        }),
        ([] @ brackets => {
            let mut child = get_child!(brackets, lexer);
            let members = call!(parse_tuple, &mut child)?;
            Ok(Atom::Array(Array{ brackets, members }))
        }),
    )
}

fn parse_literal(lexer: &mut Lexer) -> ParseResult<Literal> {
    Ok(match_tok!(lexer.next(),
        (String @ t => Literal::String(t)),
        (Float @ t => Literal::Float(t)),
        (uninit @ t => Literal::Uninit(t)),
        (true @ t => Literal::True(t)),
        (false @ t => Literal::False(t)),
        (Character @ t => Literal::Character(t)),
        (IntegerBin @ t => Literal::Binary(t)),
        (IntegerOct @ t => Literal::Octal(t)),
        (IntegerDec @ t => Literal::Decimal(t)),
        (IntegerHex @ t => Literal::Hexadecimal(t)),
        (null @ null_kw => {
            if let Token![[] #(brackets)] = lexer.lookahead() {
                lexer.next();
                Literal::NullArray(NullArray{ null_kw, brackets })
            } else { Literal::Null(null_kw) }
        }),
    ))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
enum BindingPower {
    MinPower,
    LogOr,
    LogAnd,
    BitOr,
    BitXor,
    BitAnd,
    Eq,
    Cmp,
    Shift,
    Sum,
    Prod,
    Prefix,
    // Postfix,
}

#[inline] fn parse_expression(lexer: &mut Lexer) -> ParseResult<Expression> {
    parse_expr(lexer, BindingPower::MinPower)
}

fn parse_expr(lexer: &mut Lexer, power: BindingPower) -> ParseResult<Expression> {
    let tok = lexer.lookahead();
    let mut lhs = if let Some(op) = tok.as_prefix() {
        lexer.next();
        let target = call!(parse_expr, lexer, BindingPower::Prefix).map(Box::new)?;
        Expression::Unary(Unary { op, target })
    } else {
        let atom = call!(parse_atom, lexer)?;
        Expression::Atom(atom)
    };

    loop {
        if let Some(op) = call!(parse_postfix, lexer)? {
            lhs = Expression::Unary(Unary { op, target: Box::new(lhs) });
            continue;
        }
        let tok = lexer.lookahead();
        let Some(op) = tok.as_infix() else { break };
        let binding_power = op.binding_power();
        if binding_power < power { break }
        lexer.next();
        let rhs = call!(parse_expr, lexer, binding_power).map(Box::new)?;
        lhs = Expression::Binary(Binary {
            lhs: Box::new(lhs), op, rhs
        });
    }

    Ok(lhs)
}

impl Token {
    fn as_prefix(&self) -> Option<UnaryOperator> {
        match self.clone() {
            Token![* #(t)] => Some(UnaryOperator::Dereference(t)),
            Token![*. #(t)] => Some(UnaryOperator::NonNullDereference(t)),
            Token![& #(t)] => Some(UnaryOperator::Reference(t)),
            Token![! #(t)] => Some(UnaryOperator::Not(t)),
            Token![- #(t)] => Some(UnaryOperator::Negate(t)),
            _ => None
        }
    }

    fn as_infix(&self) -> Option<BinaryOperator> {
        Some(match self.clone() {
            Token![+ #(t)] => BinaryOperator::Add(t),
            Token![- #(t)] => BinaryOperator::Subtract(t),
            Token![* #(t)] => BinaryOperator::Multiply(t),
            Token![/ #(t)] => BinaryOperator::Divide(t),
            Token![% #(t)] => BinaryOperator::Remainder(t),
            Token![== #(t)] => BinaryOperator::Equals(t),
            Token![!= #(t)] => BinaryOperator::NotEquals(t),
            Token![<= #(t)] => BinaryOperator::LessOrEqual(t),
            Token![>= #(t)] => BinaryOperator::GreaterOrEqual(t),
            Token![< #(t)] => BinaryOperator::Less(t),
            Token![> #(t)] => BinaryOperator::Greater(t),
            Token![&& #(t)] => BinaryOperator::LogicalAnd(t),
            Token![|| #(t)] => BinaryOperator::LogicalOr(t),
            Token![& #(t)] => BinaryOperator::BitwiseAnd(t),
            Token![| #(t)] => BinaryOperator::BitwiseOr(t),
            Token![^ #(t)] => BinaryOperator::BitwiseXor(t),
            Token![<< #(t)] => BinaryOperator::ShiftLeft(t),
            Token![>> #(t)] => BinaryOperator::ShiftRight(t),
            _ => return None
        })
    }
}

impl BinaryOperator {
    fn binding_power(&self) -> BindingPower {
        use BinaryOperator::*;
        match self {
            Add(_) | Subtract(_) => BindingPower::Sum,
            Multiply(_) | Divide(_) | Remainder(_) => BindingPower::Prod,
            Equals(_) | NotEquals(_) => BindingPower::Eq,
            LessOrEqual(_) | Less(_) | GreaterOrEqual(_) | Greater(_) => BindingPower::Cmp,
            LogicalAnd(_) => BindingPower::LogAnd,
            LogicalOr(_) => BindingPower::LogOr,
            BitwiseAnd(_) => BindingPower::BitAnd,
            BitwiseOr(_) => BindingPower::BitOr,
            BitwiseXor(_) => BindingPower::BitXor,
            ShiftLeft(_) | ShiftRight(_) => BindingPower::Shift
        }
    }
}

fn parse_postfix(lexer: &mut Lexer) -> ParseResult<Option<UnaryOperator>> {
    let pre = lexer.clone();
    Ok(Some(match lexer.next() {
        Token![? #(question)] => UnaryOperator::Unwrap(question),
        Token![. #(dot)] => {
            let field_name = expect_tok!(Identifier lexer.next());
            UnaryOperator::Access(Access { dot, field_name })
        },
        Token![-> #(arrow)] => {
            let field_name = expect_tok!(Identifier lexer.next());
            UnaryOperator::FieldAccess(FieldAccess { arrow, field_name })
        },
        Token![->& #(amparrow)] => {
            let field_name = expect_tok!(Identifier lexer.next());
            UnaryOperator::FieldPointer(FieldPointer { amparrow, field_name })
        },
        Token![=> #(fat_arrow)] => {
            let field_name = expect_tok!(Identifier lexer.next());
            UnaryOperator::NonNullFieldAccess(NonNullFieldAccess { fat_arrow, field_name })
        },
        Token![=>& #(fat_amparrow)] => {
            let field_name = expect_tok!(Identifier lexer.next());
            UnaryOperator::NonNullFieldPointer(NonNullFieldPointer { fat_amparrow, field_name })
        },
        Token![as #(as_kw)] => {
            let ty = call!(parse_type, lexer)?;
            UnaryOperator::Cast(Cast { as_kw, ty })
        },
        Token![() #(parens)] => {
            let mut child = get_child!(parens, lexer);
            let arguments = call!(parse_tuple, &mut child)?;
            UnaryOperator::Call(Call { parens, arguments })
        },
        Token![@ #(at)] => {
            let prefix = match_tok!(lexer.next(),
                (& @ and => Either::Left(and)),
                (? @ quest => Either::Right(quest)),
            );
            let brackets = expect_tok!([] lexer.next());
            UnaryOperator::Index(call!(parse_index_postfix, lexer, brackets, Some((at, prefix)))?)
        },
        Token![[] #(brackets)] => UnaryOperator::Index(call!(parse_index_postfix, lexer, brackets, None)?),
        _ => { *lexer = pre; return Ok(None); }
    }))
}

fn parse_index_postfix(lexer: &mut Lexer, brackets: Token![[]], prefix: Option<(Token![@], Either<Token![&], Token![?]>)>) -> ParseResult<Index> {
    let mut child = get_child!(brackets, lexer);
    let index = call!(parse_expression, &mut child).map(Box::new)?;
    expect_tok!(EOB child.next());
    Ok(Index { prefix, brackets, index })
}