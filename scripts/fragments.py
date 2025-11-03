from glob import glob
from pathlib import Path

TEST_SUITE = Path(__file__).parent.parent / "test-suite"
FRAGMENTS_DIR = TEST_SUITE / "fragments"
TEST_DIR = TEST_SUITE / "tests"


def get_fragment_paths() -> list[Path]:
    pattern = str(FRAGMENTS_DIR / "*.ks")
    manifests = str(FRAGMENTS_DIR / "*.toml")
    return [Path(p) for p in [*glob(pattern), *glob(manifests)]]


def get_tests():
    return "\n".join([Path(p).read_text() for p in TEST_DIR.glob("*.rs")])


def check_unused_fragments(tests: str, fragment_paths: list[Path]) -> list[Path]:
    unused = []
    for frag_path in fragment_paths:
        if frag_path.stem not in tests:
            unused.append(frag_path)
    return unused


if __name__ == "__main__":
    fragment_paths = get_fragment_paths()
    tests = get_tests()
    unused = check_unused_fragments(tests, fragment_paths)
    if unused:
        print("Unused fragments:")
        for p in unused:
            print(f" - {p}")
        exit(1)
    else:
        print("All fragments are used.")
