namespace foo;

// foo is fallible
operation foo() -> i32!;

// an infallible operation
operation add(a: i32, b: i32) -> i32;

// operation try_sub accepts `value` (`u32`) and `sub` (`u32`), and returns a result u32 (`u32!`)
operation try_sub(value: u32, sub: u32) -> u32!;
