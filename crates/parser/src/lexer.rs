use arcstr::{ArcStr, Substr};

use crate::tokens::Token;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileLocation {
    pub line: usize,
    pub column: usize
}

impl FileLocation {
    const DEFAULT: Self = Self { line: 1, column: 1 };

    fn advance_by(&mut self, chr: char) {
        if chr == '\n' {
            self.column = 0;
            self.line += 1;
        }
        self.column += 1;
    }

    fn advance_using(&mut self, slice: &str) {
        let mut last = slice;
        
        for slice in slice.split('\n').skip(1) {
            self.column = 1;
            self.line += 1;
            last = slice;
        }
        self.column += last.chars().count();
    }
}

impl Default for FileLocation {
    fn default() -> Self { Self::DEFAULT }
}

impl std::fmt::Debug for FileLocation {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}:{}", self.line, self.column)
    }
}

impl std::fmt::Display for FileLocation {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, fmt)
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub source: ArcStr,
    pub location: FileLocation
}

impl std::fmt::Debug for Span {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Span")
            .field("span", &(self.start .. self.end))
            .field("slice", &self.source.substr(self.start .. self.end))
            .field("location", &self.location)
            .finish()
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source.substr(self.start .. self.end))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lexer {
    source: Substr,
    location: FileLocation,
    index: usize,
    active_child: Option<Box<Self>>
}

impl std::fmt::Debug for Lexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lexer")
            .field("location", &self.location)
            .field("index", &self.index)
            .field("active_child", &self.active_child)
            .finish()
    }
}

impl Lexer {
    pub fn new(source: ArcStr) -> Lexer {
        Lexer {
            source: Substr::full(source),
            location: FileLocation::DEFAULT,
            index: 0,
            active_child: None
        }
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.index..).and_then(|s| s.chars().next())
    }

    fn bite(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.index += c.len_utf8();
        self.location.advance_by(c);
        Some(c)
    }

    fn munch(&mut self, p: char) -> bool {
        let matched = self.source[self.index..].starts_with(p);
        if matched { self.bite(); }
        matched
    }

    fn chomp(&mut self, p: &str) -> bool {
        let matched = self.source[self.index..].starts_with(p);
        if matched { self.index += p.len(); self.location.advance_using(p); }
        matched
    }

    const fn mark(&self) -> LexerLocation {
        LexerLocation { index: self.index, location: self.location }
    }

    fn span(&self, start: LexerLocation) -> Span {
        Span {
            start: start.index, end: self.index, source: self.source.parent().clone(), location: start.location
        }
    }
}

#[derive(Copy, Clone)]
struct LexerLocation {
    index: usize, location: FileLocation
}

// --- Scanning ---

const STACK_REDZONE: usize = 32 * 1024;
const STACK_SIZE: usize = 1024 * 1024;

impl Lexer {
    #[inline]
    fn scan(&mut self) -> Option<Token> {
        stacker::maybe_grow(STACK_REDZONE, STACK_SIZE, || self.next())
    }
}

impl Iterator for Lexer {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        if let Some(ref mut child) = self.active_child {
            if let Some(tok) = child.next() { return Some(tok) }
            self.active_child = None;
        }
        let loc = self.mark();
        let prebite = self.clone();
        let next = self.bite()?;
        if next.is_ascii_whitespace() {
            self.skip_whitespace();
            return self.next();
        }
        Some(match next {
            '?' => Token![? @ self.span(loc)],
            '%' => Token![% @ self.span(loc)],
            ';' => Token![; @ self.span(loc)],
            '^' => Token![^ @ self.span(loc)],
            '+' => Token![+ @ self.span(loc)],
            '*' => Token![* @ self.span(loc)],
            ',' => Token![, @ self.span(loc)],
            '.' => Token![. @ self.span(loc)],

            '-' => if self.munch('>') { 
                if self.munch('&') { Token![->& @ self.span(loc)] }
                else if self.munch('?') { Token![->? @ self.span(loc)] }
                else { Token![-> @ self.span(loc)] } 
            } else { Token![- @ self.span(loc)] },

            '<' => if self.munch('<') { Token![<< @ self.span(loc)] } 
                else if self.munch('=') { Token![<= @ self.span(loc)] } 
                else { Token![< @ self.span(loc)] },

            '>' => if self.munch('>') { Token![>> @ self.span(loc)] } 
                else if self.munch('=') { Token![>= @ self.span(loc)] } 
                else { Token![> @ self.span(loc)] },

            '|' => if self.munch('|') { Token![|| @ self.span(loc)] } 
                else { Token![| @ self.span(loc)] },

            '&' => if self.munch('&') { Token![&& @ self.span(loc)] } 
                else { Token![& @ self.span(loc)] },

            '!' => if self.munch('=') { Token![!= @ self.span(loc)] } 
                else { Token![! @ self.span(loc)] },

            ':' => if self.munch(':') { Token![:: @ self.span(loc)] } 
                else { Token![: @ self.span(loc)] },

            '=' => if self.munch('=') { Token![== @ self.span(loc)] } 
            else { Token![= @ self.span(loc)] },

            '/' => if self.munch('*') && self.skip_comment() { self.scan()? } 
            else if self.munch('/') { self.skip_line_comment(); self.scan()? } 
            else { Token![/ @ self.span(loc)] },

            '(' => 'b: {
                let mut l = self.clone();
                if !l.skip_balancing(')') { break 'b Token![OpenParenthesis @ self.span(loc)] };
                self.source = self.source.substr( .. l.index - 1);
                let child = self.clone();
                *self = l;
                Token![() @ self.span(loc); child]
            },

            '[' => 'b: {
                let mut l = self.clone();
                if !l.skip_balancing(']') { break 'b Token![OpenBracket @ self.span(loc)] };
                self.source = self.source.substr( .. l.index - 1);
                let child = self.clone();
                *self = l;
                Token![[] @ self.span(loc); child]
            },

            '{' => 'b: {
                let mut l = self.clone();
                if !l.skip_balancing('}') { break 'b Token![OpenBrace @ self.span(loc)] };
                self.source = self.source.substr( .. l.index - 1);
                let child = self.clone();
                *self = l;
                Token![{} @ self.span(loc); child]
            },

            ')' => Token![ClosedParenthesis @ self.span(loc)],
            ']' => Token![ClosedBracket @ self.span(loc)],
            '}' => Token![ClosedBrace @ self.span(loc)],

            '0' ..= '9' => {*self = prebite; self.chomp_number(loc)},
            '\'' => self.chomp_character(loc),
            '"' => self.chomp_string(loc),
            c if c.is_ascii_alphabetic() || c == '_' || unicode_ident::is_xid_start(c) => self.chomp_identifier(loc),
            
            _ => Token![Unknown @ self.span(loc)]
        })
    }
}

trait ResultExt<T> {
    fn into_inner(self) -> T;
}

impl<T> ResultExt<T> for Result<T, T> {
    fn into_inner(self) -> T {
        let (Ok(i) | Err(i)) = self;
        i
    }
}

impl Lexer {
    fn skip_balancing(&mut self, target: char) -> bool {
        let mut depth = 1;
        while depth > 0 {
            let Some(chr) = self.peek() else { return false };
            match chr {
                c if c == target => depth -= 1,
                '(' | '{' | '[' => depth += 1,
                ')' | '}' | ']' => if depth >= 1 { depth -= 1 } else { return false; },
                _ => {}
            }
            self.index += chr.len_utf8();
            self.location.advance_by(chr);
        }
        return true;
    }

    fn skip_comment(&mut self) -> bool {
        let mut l = self.clone();
        while !l.chomp("*/") {
            let Some(_) = l.bite() else { return false };
        }
        self.index = l.index;
        self.location = l.location;
        true
    }

    fn skip_line_comment(&mut self) {
        let slice = &self.source[self.index..];
        let newline_index = slice.find('\n').unwrap_or(slice.len());
        let comment = &slice[..newline_index];
        self.index += comment.len();
        self.location.advance_using(comment);
    }

    fn skip_whitespace(&mut self) {
        let slice = &self.source[self.index..];
        let non_whitespace_index = slice.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(slice.len());
        let whitespace = &slice[..non_whitespace_index];
        self.index += whitespace.len();
        self.location.advance_using(whitespace);
    }

    fn chomp_number(&mut self, loc: LexerLocation) -> Token {
        enum Format {
            Dec, Hex, Bin, Oct, Float
        }
        impl Format {
            fn allowed(&self, chr: char) -> bool {
                match self {
                    Format::Bin => chr == '0' || chr == '1',
                    Format::Hex => chr.is_ascii_hexdigit(),
                    Format::Oct => matches!(chr, '0' ..= '7'),
                    Format::Dec | Format::Float => chr.is_ascii_digit()
                }
            }
        }

        let mut format = 
            if self.chomp("0x") { Format::Hex }
            else if self.chomp("0b") { Format::Bin }
            else if self.chomp("0o") { Format::Oct }
            else { Format::Dec };
        
        while let Some(chr) = self.peek() {
            if format.allowed(chr) {
                self.bite();
            } else if matches!(format, Format::Dec) && chr == '.' {
                format = Format::Float;
                self.bite();
            } else { break; }
        }
        match format {
            Format::Bin => Token![IntegerBin @ self.span(loc)],
            Format::Oct => Token![IntegerOct @ self.span(loc)],
            Format::Dec => Token![IntegerDec @ self.span(loc)],
            Format::Hex => Token![IntegerHex @ self.span(loc)],
            Format::Float => Token![Float @ self.span(loc)]
        }
    }

    fn chomp_identifier(&mut self, loc: LexerLocation) -> Token {
        let s = loc.index;
        
        while let Some(chr) = self.peek() {
            if chr == '_' || chr.is_ascii_alphanumeric() || unicode_ident::is_xid_continue(chr) {
                self.bite();
            } else { break; }
        }

        let word = &self.source[s .. self.index];
        match word {
            "import" => Token![import @ self.span(loc)],
            "struct" => Token![struct @ self.span(loc)],
            "enum" => Token![enum @ self.span(loc)],
            "union" => Token![union @ self.span(loc)],
            "module" => Token![module @ self.span(loc)],
            "function" => Token![function @ self.span(loc)],
            "constant" => Token![constant @ self.span(loc)],
            "static" => Token![static @ self.span(loc)],
            "global" => Token![global @ self.span(loc)],
            "external" => Token![external @ self.span(loc)],
            "overload" => Token![overload @ self.span(loc)],
            "public" => Token![public @ self.span(loc)],
            "internal" => Token![internal @ self.span(loc)],
            "if" => Token![if @ self.span(loc)],
            "else" => Token![else @ self.span(loc)],
            "for" => Token![for @ self.span(loc)],
            "while" => Token![while @ self.span(loc)],
            "continue" => Token![continue @ self.span(loc)],
            "break" => Token![break @ self.span(loc)],
            "switch" => Token![switch @ self.span(loc)],
            "case" => Token![case @ self.span(loc)],
            "return" => Token![return @ self.span(loc)],
            "explode" => Token![explode @ self.span(loc)],
            "with" => Token![with @ self.span(loc)],
            "let" => Token![let @ self.span(loc)],
            "size" => Token![size @ self.span(loc)],
            "alignment" => Token![alignment @ self.span(loc)],
            "null" => Token![null @ self.span(loc)],
            "allocate" => Token![allocate @ self.span(loc)],
            "deallocate" => Token![deallocate @ self.span(loc)],
            "as" => Token![as @ self.span(loc)],
            "true" => Token![true @ self.span(loc)],
            "false" => Token![false @ self.span(loc)],
            "super" => Token![super @ self.span(loc)],
            _ => Token![Identifier @ self.span(loc)]
        }
    }

    fn chomp_character(&mut self, loc: LexerLocation) -> Token {
        self.chomp_stringlike('\'')
            .map(|_| Token![Character @ self.span(loc)])
            .map_err(|_| Token![UnterminatedCharacter @ self.span(loc)])
            .into_inner()
    }

    fn chomp_string(&mut self, loc: LexerLocation) -> Token {
        self.chomp_stringlike('"')
            .map(|_| Token![String @ self.span(loc)])
            .map_err(|_| Token![UnterminatedString @ self.span(loc)])
            .into_inner()
    }

    // Result<(), ()> is a sin but i'm too lazy to fully refactor
    fn chomp_stringlike(&mut self, target: char) -> Result<(), ()> {
        self.bite();
        let mut last_was_escape = false;
        loop {
            let chr = self.peek().ok_or(())?;
            if last_was_escape {
                last_was_escape = false;
                self.bite();
                continue;
            }
            self.bite();
            if chr == target { break Ok(()); }
            if chr == '\\' { last_was_escape = true; }
        }
    }
}