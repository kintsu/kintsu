from pathlib import Path

if __name__ == "__main__":
    f = Path(__file__).parent / "src" / "schema.rs"
    d = f.read_text()
    d = d.replace("Array<Nullable<Text>>", "Array<Text>")
    f.write_text(d)
