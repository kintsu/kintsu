from subprocess import run
from typer import Typer, Option
from glob import glob
from pathlib import Path
from json import loads

app = Typer()


@app.command()
def pub(
    dirname: str = Option(".", help="Directory to walk for packages"),
    command: str = Option("target/debug/kintsu"),
):
    dirname = Path(dirname)
    order_json = loads((dirname / "order.json").read_text())

    for f in order_json:
        p = dirname / f
        if p.is_dir():
            print(f"Publishing package in {p}...")
            run(
                [
                    command,
                    "registry",
                    "publish",
                    "-d",
                    str(p),
                    "-r",
                    "http://localhost:8000",
                ],
                check=True,
            )


if __name__ == "__main__":
    app()
