macro_rules! token_enum {
    (@second $tt2: tt) => { $tt2 };
    (@second $tt: tt $tt2: tt) => { $tt2 };
    ($($var: ident $($no: ident)? = $tt: tt $([$($others: tt)*])?),* $(,)?) => {
        #[macro_export]
        #[doc(hidden)]
        macro_rules! _Token {
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
        }

        $(
            token_enum! { @struct $($no)? $var }
        )*
    };
    (@struct extern $var: ident) => {};
    (@struct $var: ident) => {
        #[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $var {
            pub span: $crate::lexer::Span
        }

        impl From<$var> for Token {
            fn from(value: $var) -> Token {
                Token::$var(value)
            }
        }
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
    Overload = overload,
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
    Explode = explode,
    With = with,
    Let = let,
    Size = size,
    Alignment = alignment,
    Allocate = allocate,
    Deallocate = deallocate,
    As = as,
    Parent = parent,
    // Values,
    Null = null,
    True = true,
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
    ArrowAmpersand = -> [&],
    ArrowQuestion = -> [?],
    Arrow = ->,
    DoubleEqual = ==,
    Equal = =,
    Less = <,
    Greater = >,
    Plus = +,
    Minus = -,
    Asterisk = *,
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
    UnterminatedCharacter = UnterminatedCharacter

}