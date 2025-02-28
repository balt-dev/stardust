use std::{error::Error, process::ExitCode};

use parser::lexer::Lexer;

static TEST_FILE: &'static str = r#"
public struct Vector2 {
    public x: f32,
    public y: f32
}

public module Vector2 {
    import parent::Vector2;
    import std::{math::sqrt, io, io::Formatter};

    public function new(with x: f32, with y: f32) -> Vector2 {
        return struct Vector2 { x: x, y: y };
    }
    public function magnitude(with self: *constant Vector2) -> f32 {
        with x: f32 = self->x;
        with y: f32 = self->y;
        return sqrt(x * x + y * y);
    }
    public function dot(with self: *constant Vector2, with other: *constant Vector2) -> f32 {
        return self->x * other->x + self->y * other->y;
    }

    public function io::write (with f: *Formatter, with self: *constant Vector2) -> bool {
        return io::write(f, &'<')
            && io::write(f, self->x)
            && io::write(f, ", ")
            && io::write(f, self->y)
            && io::write(f, &'>');
    }
}

import std::{io, io::{stdout, Formatter, write}};

public function main(with args: u8[][]) -> i32 {
    with a: Vector2 = Vector2::new(1.0, 0.0);
    with b: Vector2 = Vector2::new(-1.0, 0.0);
    with dot: f32 = Vector2::dot(&a, &b);

    with res: bool =
        write(fmt, "Vector 1: ") &&
        write(fmt, &a) &&
        write(fmt, &'\n') &&
        write(fmt, "Vector 2: ") &&
        write(fmt, &b) &&
        write(fmt, &'\n') &&
        write(fmt, "Dot product: ") &&
        write(fmt, &dot) &&
        write(fmt, &'\n');

    return res as i32;
}
"#;

#[test]
fn lexer_test() {
    let lex = Lexer::new(TEST_FILE.into());
    println!("{lex}");
}

#[test]
fn parser_test() -> ExitCode {
    fn test() -> Result<(), Box<dyn Error>> {
        let mut lex = Lexer::new(TEST_FILE.into());
        let parsed = parser::parse::parse_module(&mut lex)?;
        println!("{parsed:#?}");
        Ok(())
    }
    match test() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}