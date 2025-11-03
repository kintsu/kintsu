namespace foo;

use bar_corp::bar::baz;

use bar_corp::bar::baz::{BazOrString};

// single line comment on type
type Abc = oneof str | i64;

/* some multiline comment. */
struct TestComplexMessage {
	// Field comment
	a: i32,
	/* Multiline field comment */
	b: i64,
	c: str,
	d?: bool,
	e?: str,
	f: BazOrString,
	g: Abc,
	h: baz::Baz
};
