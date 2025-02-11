use parser::lexer::Lexer;

#[test]
fn lexer_test() {
    let file = r#"
        public struct Vector2 {
            public x: f32,
            public y: f32
        }

        public module Vector2 {
            import super::Vector2;
            import std::{math::sqrt, io, io::Formatter};

            public function new(x: f32, y: f32) -> Vector2 {
                return Vector2 { x: x, y: y };
            }
            public function magnitude(self: *constant Vector2) -> f32 {
                with x: f32 = self->x;
                with y: f32 = self->y;
                return sqrt(x * x + y * y);
            }
            public function dot(self: *constant Vector2, other: *constant Vector2) -> f32 {
                return self->x * other->x + self->y * other->y;
            }

            public overload io::write (f: *Formatter, self: *constant Vector2) -> bool {
                return io::write(f, '<')
                    && io::write(f, self->x)
                    && io::write(f, ", ")
                    && io::write(f, self->y)
                    && io::write(f, '>');
            }
        }

        use std::{io, io::stdout};

        public function main(args: u8[][]) -> i32 {
            with a: Vector2 = Vector2::new(1.0, 0.0);
            with b: Vector2 = Vector2::new(-1.0, 0.0);
            with dot: f32 = Vector2::dot(&a, &b);
            
            io::write(&stdout->fmt, &"Vector 1: ");
            io::write(&stdout->fmt, &a);
            io::write(&stdout->fmt, &'\n');

            io::write(&stdout->fmt, &"Vector 2: ");
            io::write(&stdout->fmt, &b);
            io::write(&stdout->fmt, &'\n');

            io::write(&stdout->fmt, &"Dot product: ");
            io::write(&stdout->fmt, &dot);
            io::write(&stdout->fmt, &'\n');

            return 0;
        }
    "#;

    let lex = Lexer::new(file.into());

    for tok in lex {
        print!("{:#}\n", tok)
    }
}