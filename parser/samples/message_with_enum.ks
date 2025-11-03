#[version(1)]
namespace test;

enum IntEnum {
	Foo = 1,
	AnswerToLife = 42
};

struct TestEnumMessage {
	a: IntEnum
};
