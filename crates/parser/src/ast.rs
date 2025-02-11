use crate::tokens::Token;

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
            pub colon: Token![:],
            pub library: Token![String],
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
            pub decl: VariableDeclaration
        },
        Static {
            pub vis: Visibility,
            pub static_kw: Token![static],
            pub decl: VariableDeclaration
        },
        Global {
            pub vis: Visibility,
            pub global_kw: Token![global],
            pub decl: VariableDeclaration
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
            pub variant: Separated<Variant, Token![,]>
        },
        Function {
            pub vis: Visibility,
            pub function_kw: Token![function],
            pub name: Token![Identifier],
            pub def: FuncDef
        },
        Overload {
            pub vis: Visibility,
            pub function_kw: Token![overload],
            pub name: Path,
            pub def: FuncDef
        }
    }
}

lang_enum! {
    pub enum ExternItem {
        ExtGlobal {
            pub global_kw: Token![global],
            pub arg: Argument
        },
        ExtFunction {
            pub vis: Visibility,
            pub function_kw: Token![function],
            pub name: Token![Identifier],           
            pub parens: Token![()],
            pub arguments: Separated<Argument, Token![,]>,
            pub return_ty: Option<(Token![->], Type)>,
            pub semicolon: Token![;]
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDeclaration {
    pub arg: Argument,
    pub equal: Token![=],
    pub expr: Expression,
    pub semicolon: Token![;]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub vis: Visibility,
    pub arg: Argument
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argument {
    pub name: Token![Identifier],
    pub colon: Token![:],
    pub ty: Type
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub vis: Visibility,
    pub name: Token![Identifier],
    pub value: Option<(Token![=], Expression)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncDef {
    pub parens: Token![()],
    pub arguments: Separated<Argument, Token![,]>,
    pub return_ty: Option<(Token![->], Type)>,
    pub brace: Token![{}],
    pub statements: Vec<Statement>
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
            pub length: Option<Expression>
        },
        FunctionTy {
            pub function_kw: Token![function],
            pub parens: Token![()],
            pub args: Separated<Type, Token![,]>,
            pub return_ty: Option<(Token![->], Box<Type>)>
        }
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
        Index {
            pub target: Box<Expression>,
            pub brackets: Token![[]],
            pub index: Box<Expression>,
        },
        Call {
            pub target: Box<Expression>,
            pub parens: Token![()],
            pub arguments: Separated<Expression, Token![,]>,
        },
    }
}

// TODO
pub type Statement = ();