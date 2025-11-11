import shutil

from collections import defaultdict
from datetime import datetime
from pathlib import Path
from typing import Dict, List

import tomlkit
from tomlkit.items import Table, InlineTable
import typer

app = typer.Typer()

IGNORES = {"target", ".git", "node_modules", "vendor"}
SECTIONS = ("dependencies", "dev-dependencies", "build-dependencies")


def find_cargo_tomls(root: Path) -> List[Path]:
    return sorted(p for p in root.rglob("Cargo.toml") if not (IGNORES & set(p.parts)))


def backup_file(p: Path) -> Path:
    bak = p.with_name(p.name + ".bak")
    if not bak.exists():
        shutil.copy2(p, bak)
        return bak
    ts = datetime.utcnow().strftime("%Y%m%dT%H%M%SZ")
    bak2 = p.with_name(f"{p.name}.{ts}.bak")
    shutil.copy2(p, bak2)
    return bak2


def ensure_workspace_deps(root_doc: Table) -> Table:
    if "workspace" not in root_doc:
        root_doc["workspace"] = tomlkit.table()
    ws = root_doc["workspace"]
    if "dependencies" not in ws:
        ws["dependencies"] = tomlkit.table()
    return ws["dependencies"]


def extract_version(spec) -> str | None:
    if isinstance(spec, str):
        return spec
    if isinstance(spec, (Table, InlineTable)):
        if spec.get("path"):
            return None
        return spec.get("version")
    return None


def mutate_member_dep(dep_table: Table, dep: str, version: str):
    """Remove version from dep_table[dep] and set workspace = true in-place."""
    spec = dep_table[dep]
    if isinstance(spec, str):
        inline = tomlkit.inline_table()
        inline["workspace"] = True
        dep_table[dep] = inline
    elif isinstance(spec, (Table, InlineTable)):
        if "version" in spec:
            del spec["version"]
        spec["workspace"] = True


@app.command()
def hoist(
    root: Path = typer.Option(Path(__file__).resolve().parents[1]),
    dry_run: bool = typer.Option(False, "--dry-run"),
    verbose: bool = typer.Option(False, "-v"),
):
    """Extract versions to workspace.dependencies and set workspace=true in members."""
    root = Path(root)
    root_toml = root / "Cargo.toml"
    if not root_toml.exists():
        typer.echo(f"No Cargo.toml at {root}")
        raise typer.Exit(1)

    files = find_cargo_tomls(root)
    if root_toml in files:
        files.remove(root_toml)
    files = [root_toml] + files

    docs: Dict[Path, Table] = {p: tomlkit.parse(p.read_text()) for p in files}

    # collect deps from members
    needed = defaultdict(list)  # dep -> [(sec, p, spec), ...]
    for p in files[1:]:
        for sec in SECTIONS:
            tbl = docs[p].get(sec)
            if isinstance(tbl, Table):
                for dep, spec in tbl.items():
                    if isinstance(spec, (Table, InlineTable)) and spec.get("workspace"):
                        continue
                    needed[dep].append((sec, p, spec))

    ws_deps = ensure_workspace_deps(docs[root_toml])
    added = updated = skipped = 0

    for dep, occ in needed.items():
        # find first version
        version = None
        skip = False
        for sec, p, spec in occ:
            v = extract_version(spec)
            if v is None:
                if isinstance(spec, (Table, InlineTable)) and spec.get("path"):
                    skip = True
                    break
            elif version is None:
                version = v

        if skip or version is None:
            skipped += 1
            if verbose:
                typer.echo(f"skip {dep}")
            continue

        # add to workspace.dependencies if not present
        if dep not in ws_deps:
            ws_deps[dep] = version
            added += 1
            if verbose:
                typer.echo(f"add {dep} = {version}")

        # mutate each member occurrence
        for sec, p, spec in occ:
            sect = docs[p].get(sec)
            if isinstance(sect, Table) and dep in sect:
                mutate_member_dep(sect, dep, version)
                updated += 1

    if dry_run:
        typer.echo(f"[dry-run] added={added} updated={updated} skipped={skipped}")
        raise typer.Exit()

    # write backups and files
    modified = []
    if added:
        bak = backup_file(root_toml)
        modified.append((root_toml.relative_to(root), bak.name))
        root_toml.write_text(
            tomlkit.dumps(docs[root_toml]),
        )

    seen = set()
    for sec, p, _ in [x for occ in needed.values() for x in occ]:
        if p in seen or p == root_toml:
            continue
        seen.add(p)
        bak = backup_file(p)
        modified.append((p.relative_to(root), bak.name))
        p.write_text(tomlkit.dumps(docs[p]))

    typer.echo(f"done: added={added} updated={updated} skipped={skipped}")
    if modified:
        typer.echo("backups:")
        for f, b in modified:
            typer.echo(f"  {f} -> {b}")


@app.command()
def restore(
    root: Path = typer.Option(Path(__file__).resolve().parents[1]),
    dry_run: bool = typer.Option(False, "--dry-run"),
):
    """Restore Cargo.toml files from .bak backups."""
    root = Path(root)
    backups = sorted(root.rglob("Cargo.toml.bak"))

    if not backups:
        typer.echo("No Cargo.toml.bak files found")
        raise typer.Exit()

    restored = []
    for bak in backups:
        if IGNORES & set(bak.parts):
            continue
        orig = bak.with_name("Cargo.toml")
        if orig.exists():
            restored.append((orig.relative_to(root), bak.name))
            if not dry_run:
                shutil.copy2(bak, orig)
                typer.echo(f"restored {orig.relative_to(root)}")
        else:
            typer.echo(f"skip {bak.relative_to(root)} (no original)")

    if dry_run:
        typer.echo(f"[dry-run] would restore {len(restored)} files")
    else:
        typer.echo(f"restored {len(restored)} files")


def sort_deps_table(tbl: Table) -> Table:
    """Sort dependencies in table: kintsu* deps first (sorted), then others (sorted)."""
    if not isinstance(tbl, Table):
        return
    items = list(tbl.items())
    kintsu = sorted((k, v) for k, v in items if k.startswith("kintsu"))
    others = sorted((k, v) for k, v in items if not k.startswith("kintsu"))
    out = tomlkit.table()
    for k, v in kintsu + others:
        t = tomlkit.inline_table()
        t.update(v) if isinstance(v, Table) else None

        out[k] = t if t else v
    return out


@app.command()
def sort(
    root: Path = typer.Option(Path(__file__).resolve().parents[1]),
    dry_run: bool = typer.Option(False, "--dry-run"),
):
    """Sort dependencies in all Cargo.toml files (kintsu* first, then alphabetical)."""
    root = Path(root)
    files = find_cargo_tomls(root)

    sorted_count = 0
    for p in files:
        backup_file(p)

        doc = tomlkit.parse(p.read_text())
        modified = False

        for sec in SECTIONS:
            if sec in doc and isinstance(doc[sec], Table):
                doc[sec] = sort_deps_table(doc[sec])
                modified = True

        if "workspace" in doc and isinstance(doc["workspace"], Table):
            ws = doc["workspace"]
            if "dependencies" in ws and isinstance(ws["dependencies"], Table):
                ws["dependencies"] = sort_deps_table(ws["dependencies"])
                modified = True

            if "members" in ws and isinstance(ws["members"], list):
                ws["members"] = sorted(ws["members"])
                modified = True

        if modified:
            sorted_count += 1
            if not dry_run:
                p.write_text(
                    tomlkit.dumps(doc),
                )
                typer.echo(f"sorted {p.relative_to(root)}")

    if dry_run:
        typer.echo(f"[dry-run] would sort {sorted_count} files")
    else:
        typer.echo(f"sorted {sorted_count} files")


if __name__ == "__main__":
    app()
