macro_rules! token_enum {
    (@second $tt2: tt) => { $tt2 };
    (@second $tt: tt $tt2: tt) => { $tt2 };
    ($($var: ident = $tt: tt $([$($others: tt)*])?),* $(,)?) => {
        #[macro_export]
        #[doc(hidden)]
        macro_rules! _Token {
            $(($tt $($($others)*)? #($name: ident)) => {$crate::tokens::Token::$var($name)};)*
            $(($tt $($($others)*)? #()) => {$crate::tokens::Token::$var(_)};)*
            $(($tt $($($others)*)? #) => {$crate::tokens::TokenType::$var};)*
            $(($tt $($($others)*)?) => {$crate::tokens::$var};)*
            $(($tt $($($others)*)? @ $expr: expr) => {$crate::tokens::$var { span: $expr } .into()};)*
            $(($tt $($($others)*)? @ $expr: expr; $children: expr ) => {$crate::tokens::$var { span: $expr, children: $children } .into()};)*
        }

        pub use _Token as Token;

        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
        pub enum Token {
            $($var ( $var )),*
        }

        fn unfuck(string: &str) -> std::string::String {
            let s = string.split(|c: char| c.is_ascii_whitespace()).filter(|s| s.len() > 0);
            s.collect::<Vec<&str>>().join(" ")
        }

        impl std::fmt::Display for Token {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_> ) -> std::fmt::Result {
                match self {
                    $(
                        Self::$var( $var { span, .. }) => {
                            if f.alternate() {
                                write!(f, "[{}: {}] ", stringify!($var), span.location)?;
                            }
                            write!(f, "{}", unfuck(&span.source[span.start .. span.end]))
                        }
                    )*
                    
                }
            }
        }

        impl std::fmt::Debug for Token {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_> ) -> std::fmt::Result {
                match self {
                    $(Self::$var( $var { span, .. }) => write!(f, "{}({})", stringify!($var), span)),*
                }
            }
        }

        impl Token {
            pub fn span(&self ) -> &$crate::lexer::Span {
                match self {
                    $(Self::$var( $var { span, .. }))|* => span
                }
            }

            pub const fn ty(&self) -> TokenType {
                match self {
                    $(Token::$var(_) => TokenType::$var),*
                }
            }
        }

        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum TokenType {
            $($var),*
        }

        impl TokenType {
            pub fn into_token(self, span: $crate::lexer::Span) -> Token {
                match self {
                    $(TokenType::$var => Token::$var($var { span })),*
                }
            }
        }

        $(
            token_enum! { @struct $var }
        )*
    };
    (@struct $var: ident) => {
        #[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $var {
            pub span: $crate::lexer::Span
        }

        impl $var {
            pub const TYPE: TokenType = TokenType::$var;
        }

        impl From<$var> for Token {
            fn from(value: $var) -> Token {
                Token::$var(value)
            }
        }
    }
}

impl Token {
    pub const fn is_eob(&self) -> bool {
        matches!(self, Token::EndOfBlock(_))
    }
}

token_enum! {
    // Keywords
    Import = import,
    Struct = struct,
    Enum = enum,
    Union = union,
    Module = module,
    Function = function,
    Constant = constant,
    Static = static,
    Global = global,
    External = external,
    Public = public,
    Internal = internal,
    If = if,
    Else = else,
    For = for,
    While = while,
    Continue = continue,
    Break = break,
    Switch = switch,
    Case = case,
    Return = return,
    With = with,
    Let = let,
    Size = size,
    Alignment = alignment,
    Allocate = allocate,
    Deallocate = deallocate,
    As = as,
    Parent = parent,
    Type = type,
    Any = any,
    // Values,
    Null = null,
    True = true,
    Uninit = uninit,
    False = false,
    IntegerDec = IntegerDec,
    IntegerBin = IntegerBin,
    IntegerOct = IntegerOct,
    IntegerHex = IntegerHex,
    Float = Float,
    String = String,
    Character = Character,
    Identifier = Identifier,
    // Punctuation,
    DoubleLess = <<,
    DoubleGreater = >>,
    LessEqual = <=,
    GreaterEqual = >=,
    Arrow = ->,
    ArrowAmpersand = -> [&],
    FatArrow = =>,
    FatArrowAmpersand = => [&],
    DoubleEqual = ==,
    Equal = =,
    Less = <,
    Greater = >,
    Plus = +,
    Minus = -,
    Asterisk = *,
    AsteriskDot = * [.],
    DoubleAmpersand = &&,
    Ampersand = &,
    DoubleBar = ||,
    Bar = |,
    Carat = ^,
    Exclamation = !,
    ExclamationEqual = !=,
    Slash = /,
    Percent = %,
    Question = ?, 
    Comma = ,,
    Dot = .,
    DoubleColon = ::,
    Colon = :,
    Semicolon = ;,
    At = @,
    Parentheses = (),
    Brackets = [],
    Braces = {},

    OpenParenthesis = OpenParenthesis,
    OpenBracket = OpenBracket,
    OpenBrace = OpenBrace,
    ClosedParenthesis = ClosedParenthesis,
    ClosedBracket = ClosedBracket,
    ClosedBrace = ClosedBrace,
    Unknown = Unknown,
    UnterminatedString = UnterminatedString,
    UnterminatedCharacter = UnterminatedCharacter,
    EndOfBlock = EOB

}