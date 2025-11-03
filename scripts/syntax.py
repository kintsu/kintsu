from json import loads
from pathlib import Path
import polars as pl

ROOT = Path(__file__).parent.parent / "syntax.json"

def get_src() -> dict:
    return loads(ROOT.read_text())

def path_for(kind: str):
    return ROOT.parent / "docs/src/syntax"/ f"{kind}.md"

def walk_node(root: dict, kind: str):
    d = pl.DataFrame(
        root[kind]
    ).select(
        ("`" + pl.col("token") + "`").alias("Token"),
        pl.col("description").alias("Description"),
    )
    return d.to_pandas().to_markdown(index=False)

NODES = ["builtin", "keywords", "tokens"]

if __name__ == "__main__":
    root = get_src()

    for node in NODES:
        out = path_for(node)
        tt = walk_node(root, node)
        out.write_text(
            f"# {node.title()}\n{tt}",
        )
