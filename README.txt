Datatypes:
- bool
- u8-64
- i8-64
- uint, int (pointer-sized)
- f32, f64
- T[N], for some type T and uint N - constant length array
- *? - a pointer to nothing in particular
- *T, for some type T - a pointer to a value of type T
- T[], for some type T - a pointer to a dynamically sized array, essentially just a struct containing a uint and arbitrarily many T's
- ?[] - a dynamically sized array of nothing in particular, array equivalent of *?
- (A, B, ...) - a pointer to a function, taking types and returning nothing
- (A, B, ...) -> C - a pointer to a function, taking types and returning something

Items:
- import
    Importing from a module path
- struct
    Product data type, stores each member one after another padded to their respective alignments
    A struct of {u8, u16, u8} would be larger than one of {u16, u8, u8}
- enum
    Datatype represented by an integer that may only be of a specific set of values
    An enum at runtime outside of this set of values is UB
- union
    Overlapping memory representing types
- module
    Essentially just namespacing
- function
    Call it with arguments, get back a value (or not)
- constant
    Value that's copy/pasted into every occurrence
- static
    Value that you can only take a pointer to, stored in read-only memory
- global
    Value that you can only take a pointer to, stored in read-write memory

Statements:
- if/else
- for
- while
- continue
- break
- switch (may only be used on enums, compiles to a binary search)
- return
- explode (immediate UB if this is encountered, similar to Rust's unreachable_unchecked())
- with <ident> = <expr>; (constant variable initialization)
- let <ident> = <expr>; (mutable variable initialization)
- <expr> <binop> = <expr>; (in-place operation, desugars to exactly <expr> = <expr> <binop <expr>;)
- <expr>; (just an expression)
- deallocate <expr>; (deallocate a *T from the heap)
- deallocate[] <expr>; (deallocate a T[] from the heap)
- { <statement>... } (scoped block)

Expressions:
- <expr> <binop> <expr> (binary operations)
- <unop> <expr>, <expr> <unop> (unary operations)
- <path> (variable names)
- size <type> (get the size of a type as a uptr)
- alignment <type> (get the alignment of a type as a uptr)
- null (a *? with a value of 0)
- null[] (a ?[] with a length and value of 0)
- allocate <type> (allocate space for a T on the heap and return a pointer to it)
- allocate[<length>] <type> (allocate space for <length> T's on the heap consecutively, then return a T[] with the specified length and pointing to it)
- <expr> as <type> (typecasting, e.g. a 64-bit enum into a u64, or a *u32 into an *f32)
- <literal> (things like 5.0, true, "Hello" - strings are just a u8[], being 8-bit clean)