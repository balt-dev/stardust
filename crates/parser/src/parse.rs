use std::fmt::Display;

use either::Either;

use crate::{ast::*, lexer::{Lexer, Span}, tokens::{Token, TokenType}};


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

impl ParseError {
    pub fn span(&self) -> &Span {
        match self {
            ParseError::TokenMismatch { found, .. } => found.span(),
            ParseError::NoChild { parent } => parent.span(),
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: this should be prettier
        let span = self.span();
        writeln!(f, "parsing error at {}:\n{span:#}", span.location)?;
        match self {
            ParseError::TokenMismatch { expected, found } => {
                write!(f, "mismatched token {found} - expected ")?;
                let mut iter = expected.iter().peekable();
                while let Some(t) = iter.next() {
                    write!(f, "{t}")?;
                    if iter.peek().is_some() { write!(f, ", ")?; }
                }
                Ok(())
            },
            ParseError::NoChild { parent } =>
                write!(f, "expected token {parent} to be a delimited group")
        }
    }
}

impl std::error::Error for ParseError {}

pub type ParseResult<T> = Result<T, ParseError>;

pub fn parse_module(lexer: &mut Lexer) -> ParseResult<Vec<Item>> {
    let mut items = vec![];
    while !lexer.lookahead().is_eob() {
        items.push(call!(parse_item, lexer)?)
    }
    Ok(items)
}

fn parse_item(lexer: &mut Lexer) -> ParseResult<Item> {
    let vis = call!(parse_vis, lexer);
    Ok(match_tok! {lexer.next(),
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
            let child = get_child!(brace, lexer);
            let fields = parse_sep(
                child,
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
            let child = get_child!(brace, lexer);
            let fields = parse_sep(
                child,
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
            let child = get_child!(brace, lexer);
            let variants = parse_sep(
                child,
                parse_variant,
                |lex| Ok(expect_tok!(, lex.next())),
                true
            )?;
            Item::Enum(Enum{
                vis, enum_kw, name, colon, repr, brace, variants
            })
        }),
        (function @ function_kw => {
            let name = call!(parse_path, lexer)?;
            let definition = call!(parse_function_definition, lexer)?;
            Item::Function(Function {
                vis, function_kw, name, definition
            })
        }),
    })
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
            let child = get_child!(brace, lexer);
            let parts = parse_sep(
                child,
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
    let child = get_child!(parens, lexer);
    let arguments = parse_sep(
        child,
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
        (any @ tok => { Type::Arbitrary(tok) }),
        (function @ function_kw => {
            let parens = expect_tok!(() lexer.next());
            let child = get_child!(parens, lexer);
            let arguments = parse_sep(
                child,
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

fn parse_sep<T, S>(mut lexer: Lexer, el: fn(&mut Lexer) -> ParseResult<T>, sep: fn(&mut Lexer) -> ParseResult<S>, allow_trailing: bool) -> ParseResult<Separated<T, S>> {
    let mut arguments = Separated::default();
    loop {
        if allow_trailing {
            if let Token![EOB #()] = lexer.lookahead() { break; }
        }
        let expr = call!(el, &mut lexer)?;
        let comma = match lexer.lookahead() {
            Token::EndOfBlock(_) => {
                arguments.end = Some(Box::new(expr));
                break;
            },
            _ => call!(sep, &mut lexer)?
        };
        arguments.parts.push((expr, comma))
    }
    Ok(arguments)
}

#[inline] fn parse_tuple(lexer: Lexer) -> ParseResult<Separated<Expression, Token![,]>> {
    parse_sep(lexer, parse_expression, |lex| Ok(expect_tok!(, lex.next())), true)
}

#[inline] fn parse_path(lexer: &mut Lexer) -> ParseResult<Path> {
    let mut parts = Separated::default();
    loop {
        let ident = expect_tok!(Identifier lexer.next());
        let dc = match lexer.lookahead() {
            Token![:: #(dc)] => dc,
            _ => { parts.end = Some(Box::new(ident)); break }
        };
        lexer.next();
        parts.parts.push((ident, dc))
    }
    Ok(parts)
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
    let pre = lexer.clone();
    match_tok!(lexer.next(),
        [default => { *lexer = pre; call!(parse_literal, lexer).map(Atom::Literal)}],
        (Identifier => { *lexer = pre; call!(parse_path, lexer).map(Atom::Path) }),
        (size @ size_kw => call!(parse_type, lexer).map(|ty| Atom::Size(Size { size_kw, ty }))),
        (alignment @ alignment_kw => call!(parse_type, lexer).map(|ty| Atom::Alignment(Alignment { alignment_kw, ty }))),
        (allocate @ allocate_kw => {
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
            let child = get_child!(brackets, lexer);
            let members = call!(parse_tuple, child)?;
            Ok(Atom::Array(Array{ brackets, members }))
        }),
        (struct @ struct_kw => call!(parse_construct, lexer, Either::Left(struct_kw)).map(Atom::Construct)),
        (union @ union_kw => call!(parse_construct, lexer, Either::Right(union_kw)).map(Atom::Construct)),
    )
}

fn parse_construct(lexer: &mut Lexer, kw: Either<Token![struct], Token![union]>) -> ParseResult<Construct> {
    let name = call!(parse_path, lexer)?;
    let braces = expect_tok!({} lexer.next());
    let child = get_child!(braces, lexer);
    let members = parse_sep(
        child,
        |lex| {
            let name = expect_tok!(Identifier lex.next());
            let colon = expect_tok!(: lex.next());
            let value = call!(parse_expression, lex)?;
            Ok(FieldValue { name, colon, value })
        }, |lex| Ok(expect_tok!(, lex.next())), true
    )?;
    Ok(Construct { kw, name, braces, members })
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

fn parse_mutate_operator(lexer: &mut Lexer) -> ParseResult<MutateOperator> {
    Ok(match_tok!(lexer.next(),
        (= @ t => MutateOperator::Assign(t)),
        (+= @ t => MutateOperator::AddAssign(t)),
        (-= @ t => MutateOperator::SubtractAssign(t)),
        (*= @ t => MutateOperator::MultiplyAssign(t)),
        (/= @ t => MutateOperator::DivideAssign(t)),
        (%= @ t => MutateOperator::RemainderAssign(t)),
        (&& [=] @ t => MutateOperator::LogicalAndAssign(t)),
        (|| [=] @ t => MutateOperator::LogicalOrAssign(t)),
        (&= @ t => MutateOperator::BitwiseAndAssign(t)),
        (|= @ t => MutateOperator::BitwiseOrAssign(t)),
        (^= @ t => MutateOperator::BitwiseXorAssign(t)),
        (<<= @ t => MutateOperator::ShiftLeftAssign(t)),
        (>>= @ t => MutateOperator::ShiftRightAssign(t)),
    ))
}

fn parse_function_definition(lexer: &mut Lexer) -> ParseResult<FuncDef> {
    let header = call!(parse_function_header, lexer)?;
    let brace = expect_tok!({} lexer.next());
    let mut child = get_child!(brace, lexer);
    let mut statements = vec![];
    while !matches!(child.lookahead(), Token![EOB #()]) {
        statements.push(call!(parse_statement, &mut child)?);
    }
    Ok(FuncDef {
        header, brace, statements
    })
}

fn parse_statement(lexer: &mut Lexer) -> ParseResult<Statement> {
    let pre = lexer.clone();
    Ok(match_tok!(lexer.next(), 
        [default => {
            *lexer = pre;
            let pre = lexer.clone();
            if let Ok(item) = call!(parse_item, lexer) {
                return Ok(Statement::StItem(item))
            }
            *lexer = pre;
            let lvalue = call!(parse_expression, lexer)?;
            let pre = lexer.clone();
            let Ok(binop) = call!(parse_mutate_operator, lexer) else {
                *lexer = pre;
                let semicolon = expect_tok!(; lexer.next());
                return Ok(Statement::StExpression(StExpression {
                    expr: lvalue, semicolon
                }))
            };
            let value = call!(parse_expression, lexer)?;
            let semicolon = expect_tok!(; lexer.next());
            Statement::Mutate(Mutate {
                lvalue, binop, value, semicolon
            })
        }],
        (; @ semi => Statement::Pass(semi)),
        (let @ let_kw => {
            let decl = call!(parse_variable_decl, lexer)?;
            Statement::Init(Init {
                kw: Either::Left(let_kw), decl
            })
        }),
        (with @ with_kw => {
            let decl = call!(parse_variable_decl, lexer)?;
            Statement::Init(Init {
                kw: Either::Right(with_kw), decl
            })
        }),
        (deallocate @ deallocate_kw => {
            let brackets = match lexer.lookahead() {
                Token![[] #(brackets)] => { lexer.next(); Some(brackets) },
                _ => None
            };
            let value = call!(parse_expression, lexer)?;
            let semicolon = expect_tok!(; lexer.next());
            Statement::Deallocate(Deallocate {
                deallocate_kw, brackets, value, semicolon
            })
        }),
        ({} @ braces => {
            let mut child = get_child!(braces, lexer);
            let mut body = vec![];
            while !matches!(child.lookahead(), Token![EOB #()]) {
                body.push(call!(parse_statement, &mut child)?);
            }
            Statement::Block(Block {
                braces, body
            })
        }),
        (continue @ continue_kw => {
            let semicolon = expect_tok!(; lexer.next());
            Statement::Continue(Continue {continue_kw, semicolon })
        }),
        (break @ break_kw => {
            let semicolon = expect_tok!(; lexer.next());
            Statement::Break(Break {break_kw, semicolon })
        }),
        (return @ return_kw => {
            let payload = match lexer.lookahead() {
                Token![; #()] => None,
                _ => Some(call!(parse_expression, lexer)?)
            };
            let semicolon = expect_tok!(; lexer.next());
            Statement::Return(Return {return_kw, payload, semicolon })
        }),
        (loop @ loop_kw => {
            let body = call!(parse_statement, lexer).map(Box::new)?;
            Statement::Loop(Loop {loop_kw, body })
        }),
        (while @ while_kw => {
            let condition = call!(parse_expression, lexer)?;
            let body = call!(parse_statement, lexer).map(Box::new)?;
            Statement::While(While {while_kw, condition, body })
        }),
        (if @ if_kw => {
            todo!()
        }),
        (for @ for_kw => {
            todo!()
        }),
        (switch @ switch_kw => {
            todo!()
        }),
    ))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
enum BindingPower {
    // Left-associative ones are RL, right-associative are LR.
    // It's a bit confusing, but #[derive(Ord)] works that way. :P
    MinPower,
    LogOrR, LogOrL,
    LogAndR, LogAndL,
    BitOrR, BitOrL,
    BitXorR, BitXorL,
    BitAndR, BitAndL,
    EqR, EqL,
    CmpR, CmpL,
    ShiftR, ShiftL,
    SumR, SumL,
    ProdR, ProdL,
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
        let [lp, rp] = op.binding_power();
        if lp < power { break }
        lexer.next();
        let rhs = call!(parse_expr, lexer, rp).map(Box::new)?;
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
    fn binding_power(&self) -> [BindingPower; 2] {
        use BinaryOperator::*;
        use BindingPower::*;
        match self {
            Add(_) | Subtract(_) => [SumL, SumR],
            Multiply(_) | Divide(_) | Remainder(_) => [ProdL, ProdR],
            Equals(_) | NotEquals(_) => [EqL, EqR],
            LessOrEqual(_) | Less(_) | GreaterOrEqual(_) | Greater(_) => [CmpL, CmpR],
            LogicalAnd(_) => [LogAndL, LogAndR],
            LogicalOr(_) => [LogOrL, LogOrR],
            BitwiseAnd(_) => [BitAndL, BitAndR],
            BitwiseOr(_) => [BitOrL, BitOrR],
            BitwiseXor(_) => [BitXorL, BitXorR],
            ShiftLeft(_) | ShiftRight(_) => [ShiftL, ShiftR],
        }
    }
}

fn parse_postfix(lexer: &mut Lexer) -> ParseResult<Option<UnaryOperator>> {
    let pre = lexer.clone();
    Ok(Some(match lexer.next() {
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
        Token![as #(as_kw)] => {
            let ty = call!(parse_type, lexer)?;
            UnaryOperator::Cast(Cast { as_kw, ty })
        },
        Token![() #(parens)] => {
            let child = get_child!(parens, lexer);
            let arguments = call!(parse_tuple, child)?;
            UnaryOperator::Call(Call { parens, arguments })
        },
        Token![@ #(at)] => {
            let brackets = expect_tok!([] lexer.next());
            UnaryOperator::Index(call!(parse_index_postfix, lexer, brackets, Some(at))?)
        },
        Token![[] #(brackets)] => UnaryOperator::Index(call!(parse_index_postfix, lexer, brackets, None)?),
        _ => { *lexer = pre; return Ok(None); }
    }))
}

fn parse_index_postfix(lexer: &mut Lexer, brackets: Token![[]], prefix: Option<Token![@]>) -> ParseResult<Index> {
    let mut child = get_child!(brackets, lexer);
    let index = call!(parse_expression, &mut child).map(Box::new)?;
    expect_tok!(EOB child.next());
    Ok(Index { prefix, brackets, index })
}