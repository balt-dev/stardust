use crate::tokens::Token;
use either::Either;

macro_rules! lang_enum {
    (pub enum $name: ident {
        $($var: ident $tt: tt),* $(,)?
    }) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum $name {
            $($var (lang_enum!(@member $var $tt))),*
        }
        
        $(
            lang_enum!{@single $var $tt}
        )*
    };
    (@single $var: ident {$($tt: tt)*}) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $var {$($tt)*}
    };
    (@single $var: ident ($($tt: tt)*)) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $var ($($tt)*);
    };
    (@single $var: ident [$($_: tt)*]) => {};
    (@member $var: ident {$($_: tt)*}) => {$var};
    (@member $var: ident ($($_: tt)*)) => {$var};
    (@member $var: ident []) => {$var};
    (@member $_: ident [$($tt: tt)*]) => {$($tt)*};
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Separated<T, S> {
    pub parts: Vec<(T, S)>,
    pub end: Option<Box<T>>
}

impl<T, S> IntoIterator for Separated<T, S> {
    type Item = (T, Option<S>);

    // This is dumb, but the compiler made me do it. :/
    type IntoIter = 
        std::iter::Chain<
            std::iter::Map<std::vec::IntoIter<(T, S)>, fn((T, S)) -> (T, Option<S>)>,
            std::iter::Map<std::option::IntoIter<Box<T>>, fn(Box<T>) -> (T, Option<S>)>
        >;

    fn into_iter(self) -> Self::IntoIter {
        self.parts.into_iter()
            .map((|(t, s)| (t, Some(s))) as fn((T, S)) -> (T, Option<S>))
            .chain(self.end.into_iter().map((|v| (*v, None)) as fn(Box<T>) -> (T, Option<S>)))
    }
}

impl<T, S> Default for Separated<T, S> {
    fn default() -> Self {
        Self { parts: vec![], end: None }
    }
}

pub type Path = Separated<Token![Identifier], Token![::]>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Internal(Token![internal]),
    Public(Token![public])
}

lang_enum! {
    pub enum ImportPath {
        ModFork {
            pub brace: Token![{}],
            pub parts: Separated<ImportPath, Token![,]>
        },
        ModParent {
            pub parent_kw: Token![parent],
            pub double_colon: Token![::],
            pub next: Box<ImportPath>
        },
        ModPath {
            pub name: Token![Identifier],
            pub double_colon: Token![::],
            pub next: Box<ImportPath>
        },
        ModEnd [ Token![Identifier] ]
    }
}

lang_enum! {
    pub enum Item {
        Import {
            pub vis: Visibility,
            pub import_kw: Token![import],
            pub path: ImportPath,
            pub semicolon: Token![;]
        },
        ExternalModule {
            pub vis: Visibility,
            pub external_kw: Token![external],
            pub module_kw: Token![module],
            pub name: Token![Identifier],
            pub brace: Token![{}],
            pub items: Vec<ExternItem>
        },
        Module {
            pub vis: Visibility,
            pub module_kw: Token![module],
            pub name: Token![Identifier],
            pub brace: Token![{}],
            pub items: Vec<Item>
        },
        Constant {
            pub vis: Visibility,
            pub constant_kw: Token![constant],
            pub decl: VariableDeclaration,
            pub semicolon: Token![;]
        },
        Static {
            pub vis: Visibility,
            pub static_kw: Token![static],
            pub decl: VariableDeclaration,
            pub semicolon: Token![;]
        },
        Global {
            pub vis: Visibility,
            pub global_kw: Token![global],
            pub decl: VariableDeclaration,
            pub semicolon: Token![;]
        },
        Struct {
            pub vis: Visibility,
            pub struct_kw: Token![struct],
            pub name: Token![Identifier],
            pub brace: Token![{}],
            pub fields: Separated<Field, Token![,]>
        },
        Union {
            pub vis: Visibility,
            pub union_kw: Token![union],
            pub name: Token![Identifier],
            pub brace: Token![{}],
            pub fields: Separated<Field, Token![,]>
        },
        Enum {
            pub vis: Visibility,
            pub enum_kw: Token![enum],
            pub name: Token![Identifier],
            pub colon: Token![:],
            pub repr: Type,
            pub brace: Token![{}],
            pub variants: Separated<Variant, Token![,]>
        },
        Function {
            pub vis: Visibility,
            pub function_kw: Token![function],
            pub name: Path,
            pub definition: FuncDef
        }
    }
}

lang_enum! {
    pub enum ExternItem {
        ExtGlobal {
            pub global_kw: Token![global],
            pub argument: Argument,
            pub semicolon: Token![;]
        },
        ExtType {
            pub type_kw: Token![type],
            pub name: Token![Identifier],
            pub semicolon: Token![;]
        },
        ExtFunction {
            pub function_kw: Token![function],
            pub header: FuncHeader,
            pub semicolon: Token![;]
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDeclaration {
    pub argument: Argument,
    pub equal: Token![=],
    pub expr: Expression,
    pub semicolon: Token![;]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub vis: Visibility,
    pub argument: Argument
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argument {
    pub name: Token![Identifier],
    pub ty: Option<(Token![:], Type)>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub name: Token![Identifier],
    pub value: Option<(Token![=], Expression)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldValue {
    pub name: Token![Identifier],
    pub colon: Token![:], 
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncDef {
    pub header: FuncHeader,
    pub brace: Token![{}],
    pub statements: Vec<Statement>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncHeader {
    pub parens: Token![()],
    pub arguments: Separated<(Either<Token![let], Token![with]>, Argument), Token![,]>,
    pub return_ty: Option<(Token![->], Type)>,
}

lang_enum! {
    pub enum Type {
        Path [ Path ],
        PointerTy {
            pub asterisk: Token![*],
            pub constant_kw: Option<Token![constant]>,
            pub target: Box<Type>
        },
        ArrayTy {
            pub target: Box<Type>,
            pub brackets: Token![[]],
            pub length: Option<Box<Expression>>
        },
        FunctionTy {
            pub function_kw: Token![function],
            pub parens: Token![()],
            pub arguments: Separated<Type, Token![,]>,
            pub return_ty: Option<(Token![->], Box<Type>)>
        },
        Arbitrary [ Token![any] ],
    }
}

lang_enum!{
    pub enum Expression {
        Binary {
            pub lhs: Box<Expression>,
            pub op: BinaryOperator,
            pub rhs: Box<Expression>,
        },
        Unary {
            pub op: UnaryOperator,
            pub target: Box<Expression>,
        },
        Atom []
    }
}

lang_enum!{
    pub enum BinaryOperator {
        Add [ Token![+] ],
        Subtract [ Token![-] ],
        Multiply [ Token![*] ],
        Divide [ Token![/] ],
        Remainder [ Token![%] ],
        Equals [ Token![==] ],
        NotEquals [ Token![!=] ],
        LessOrEqual [ Token![<=] ],
        GreaterOrEqual [ Token![>=] ],
        Less [ Token![<] ],
        Greater [ Token![>] ],
        LogicalAnd [ Token![&&] ],
        LogicalOr [ Token![||] ],
        BitwiseAnd [ Token![&] ],
        BitwiseOr [ Token![|] ],
        BitwiseXor [ Token![^] ],
        ShiftLeft [ Token![<<] ],
        ShiftRight [ Token![>>] ],
    }
}

lang_enum!{
    pub enum MutateOperator {
        Assign [ Token![=] ],
        AddAssign [ Token![+=] ],
        SubtractAssign [ Token![-=] ],
        MultiplyAssign [ Token![*=] ],
        DivideAssign [ Token![/=] ],
        RemainderAssign [ Token![%=] ],
        LogicalAndAssign [ Token![&&=] ],
        LogicalOrAssign [ Token![||=] ],
        BitwiseAndAssign [ Token![&=] ],
        BitwiseOrAssign [ Token![|=] ],
        BitwiseXorAssign [ Token![^=] ],
        ShiftLeftAssign [ Token![<<=] ],
        ShiftRightAssign [ Token![>>=] ],
    }
}

lang_enum! {
    pub enum UnaryOperator {
        Dereference [ Token![*] ],
        Reference [ Token![&] ],
        Not [ Token![!] ],
        Negate [ Token![-] ],

        Access {
            pub dot: Token![.],
            pub field_name: Token![Identifier]
        },
        FieldAccess {
            pub arrow: Token![->],
            pub field_name: Token![Identifier]
        },
        FieldPointer {
            pub amparrow: Token![->&],
            pub field_name: Token![Identifier]
        },
        Cast {
            pub as_kw: Token![as],
            pub ty: Type,
        },
        Call {
            pub parens: Token![()],
            pub arguments: Separated<Expression, Token![,]>,
        },
        Index {
            pub prefix: Option<Token![@]>,
            pub brackets: Token![[]],
            pub index: Box<Expression>,
        }
    }
}

lang_enum! {
    pub enum Atom {
        Path [],
        Literal [],
        Size {
            pub size_kw: Token![size],
            pub ty: Type
        },
        Alignment {
            pub alignment_kw: Token![alignment],
            pub ty: Type
        },
        Allocate {
            pub allocate_kw: Token![allocate],
            pub ty: Type
        },
        AllocateArray {
            pub allocate_kw: Token![allocate],
            pub brackets: Token![[]],
            pub length: Box<Expression>,
            pub ty: Type
        },
        Array {
            pub brackets: Token![[]],
            pub members: Separated<Expression, Token![,]>,
        },
        Construct {
            pub kw: Either<Token![struct], Token![union]>,
            pub name: Path,
            pub braces: Token![{}],
            pub members: Separated<FieldValue, Token![,]>,
        },
    }
}

lang_enum! {
    pub enum Literal {
        String [ Token![String] ],
        Float [ Token![Float] ],
        Null [ Token![null] ],
        Uninit [ Token![uninit] ],
        True [ Token![true] ],
        False [ Token![false] ],
        NullArray {
            pub null_kw: Token![null],
            pub brackets: Token![[]],
        },
        Character [ Token![Character] ],
        Binary [ Token![IntegerBin] ],
        Octal [ Token![IntegerOct] ],
        Decimal [ Token![IntegerDec] ],
        Hexadecimal [ Token![IntegerHex] ],
    }
}

lang_enum! {
    pub enum Statement {
        StItem [ Item ],
        Init {
            pub kw: Either<Token![let], Token![with]>,
            pub decl: VariableDeclaration
        },
        Mutate {
            pub lvalue: Expression,
            pub binop: MutateOperator,
            pub value: Expression,
            pub semicolon: Token![;],
        },
        Deallocate {
            pub deallocate_kw: Token![deallocate],
            pub brackets: Option<Token![[]]>,
            pub value: Expression,
            pub semicolon: Token![;],
        },
        If {
            pub blocks: Separated<IfBlock, Token![else]>,
            pub fallback: Box<Statement>
        },
        Switch {
            pub switch_kw: Token![switch],
            pub value: Expression,
            pub braces: Token![{}],
            pub cases: Separated<SwitchCase, Token![,]>
        },
        Block {
            pub braces: Token![{}],
            pub body: Vec<Statement>
        },
        For {
            pub for_kw: Token![for],
            pub initializer: Box<Statement>,
            pub condition: Option<Expression>,
            pub semicolon: Token![;],
            pub iterator: Option<Expression>,
            pub body: Box<Statement>
        },
        While {
            pub while_kw: Token![while],
            pub condition: Expression,
            pub body: Box<Statement>
        },
        Loop {
            pub loop_kw: Token![loop],
            pub body: Box<Statement>
        },
        Continue {
            pub continue_kw: Token![continue],
            pub semicolon: Token![;],
        },
        Break {
            pub break_kw: Token![break],
            pub semicolon: Token![;],
        },
        Return {
            pub return_kw: Token![return],
            pub payload: Option<Expression>,
            pub semicolon: Token![;],
        },
        StExpression {
            pub expr: Expression,
            pub semicolon: Token![;]
        },
        Pass [ Token![;] ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfBlock {
    pub if_kw: Token![if],
    pub condition: Expression,
    pub block: Box<Statement>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchCase {
    pub case_kw: Token![case],
    pub value: Expression,
    pub colon: Token![:],
    pub block: Box<Statement>
}