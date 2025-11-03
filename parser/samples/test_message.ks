/* namespace comments (test) */
#![version(1)]
namespace test;

use other_ns;

// single line comment on type
type Abc = oneof str | i32;

/* some multiline comment. */
struct TestComplexMessage {
	// Field comment
	a: i32,
	/* Multiline field comment */
	b: i64,
	c: str,
	d?: bool,
	e?: str,
	f: oneof str | f32,
	g: Abc,
	h: test::Abc
};
