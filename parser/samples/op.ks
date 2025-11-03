#![err(MyError)]
namespace foo;

error MyError {
    Unknown {
        desc: str
    },
    Code(i32)
};

// foo is fallible
operation foo() -> i32!;

// an infallible operation
operation add(a: i32, b: i32) -> i32;

// operation try_sub accepts `value` (`i32`) and `sub` (`i32`), and returns a result i32 (`i32!`)
operation try_sub(value: i32, sub: i32) -> i32!;
