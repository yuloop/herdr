"""Validate that literal rust-i18n keys exist in every shipped locale.

rust-i18n deliberately falls back to the key name when a translation is
missing. That is useful at runtime, but it also means a misspelled namespace
can compile and pass Clippy. This check keeps the English and Simplified
Chinese catalogs in parity and verifies every ``t!("...")`` call under
``src/`` against both catalogs without requiring a host-specific Rust build.

The locale files use ordinary YAML mappings. The parser below intentionally
accepts only mapping keys and scalar values; unsupported YAML structures fail
closed so the gate cannot silently overlook translations after a catalog
format change.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path


DEFAULT_SOURCE_ROOT = Path("src")
DEFAULT_LOCALES = (Path("locales/en.yml"), Path("locales/zh.yml"))

RUST_T_CALL_RE = re.compile(r"\bt!\s*\(")
RUST_LITERAL_KEY_RE = re.compile(
    r'\bt!\s*\(\s*"(?P<key>[A-Za-z0-9_.-]+)"',
    flags=re.MULTILINE,
)
YAML_MAPPING_RE = re.compile(
    r"^(?P<indent> *)(?P<key>[A-Za-z0-9_-]+):(?P<value>.*)$"
)


@dataclass(frozen=True)
class TranslationReference:
    key: str
    path: Path
    line: int


def parse_locale(path: Path) -> tuple[set[str], list[str]]:
    """Return leaf key paths and structural errors for one locale mapping."""

    keys: set[str] = set()
    seen_nodes: set[str] = set()
    errors: list[str] = []
    parents: list[tuple[int, str]] = []

    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if "\t" in line[: len(line) - len(line.lstrip())]:
            errors.append(f"{path}:{line_number}: tabs are not valid locale indentation")
            continue

        match = YAML_MAPPING_RE.match(line)
        if match is None:
            errors.append(
                f"{path}:{line_number}: unsupported locale syntax; expected a YAML mapping"
            )
            continue

        indent = len(match.group("indent"))
        while parents and parents[-1][0] >= indent:
            parents.pop()

        key = match.group("key")
        dotted = ".".join([*(parent for _, parent in parents), key])
        if dotted in seen_nodes:
            errors.append(f"{path}:{line_number}: duplicated locale key {dotted!r}")
        seen_nodes.add(dotted)

        value = match.group("value").strip()
        if value:
            keys.add(dotted)
        else:
            parents.append((indent, key))

    return keys, errors


def extract_translation_references(path: Path) -> tuple[list[TranslationReference], list[str]]:
    """Extract literal ``t!`` keys and reject calls that cannot be checked."""

    text = path.read_text(encoding="utf-8")
    references: list[TranslationReference] = []
    errors: list[str] = []
    literal_starts = {match.start() for match in RUST_LITERAL_KEY_RE.finditer(text)}

    for call in RUST_T_CALL_RE.finditer(text):
        line = text.count("\n", 0, call.start()) + 1
        if call.start() not in literal_starts:
            errors.append(
                f"{path}:{line}: translation key must be a plain string literal so it can be validated"
            )

    for match in RUST_LITERAL_KEY_RE.finditer(text):
        references.append(
            TranslationReference(
                key=match.group("key"),
                path=path,
                line=text.count("\n", 0, match.start()) + 1,
            )
        )

    return references, errors


def check_i18n_keys(
    project_root: Path,
    source_root: Path = DEFAULT_SOURCE_ROOT,
    locale_paths: tuple[Path, ...] = DEFAULT_LOCALES,
) -> list[str]:
    """Return all catalog parity, source syntax, and missing-key errors."""

    errors: list[str] = []
    catalogs: dict[Path, set[str]] = {}
    for relative_path in locale_paths:
        path = project_root / relative_path
        keys, locale_errors = parse_locale(path)
        catalogs[relative_path] = keys
        errors.extend(locale_errors)

    if catalogs:
        reference_path = locale_paths[0]
        reference_keys = catalogs[reference_path]
        for locale_path in locale_paths[1:]:
            locale_keys = catalogs[locale_path]
            for key in sorted(reference_keys - locale_keys):
                errors.append(
                    f"{project_root / locale_path}: missing locale key {key!r} "
                    f"present in {reference_path}"
                )
            for key in sorted(locale_keys - reference_keys):
                errors.append(
                    f"{project_root / locale_path}: extra locale key {key!r} "
                    f"not present in {reference_path}"
                )

    references: list[TranslationReference] = []
    for path in sorted((project_root / source_root).rglob("*.rs")):
        file_references, source_errors = extract_translation_references(path)
        references.extend(file_references)
        errors.extend(source_errors)

    for reference in references:
        for locale_path, keys in catalogs.items():
            if reference.key not in keys:
                errors.append(
                    f"{reference.path}:{reference.line}: missing translation key "
                    f"{reference.key!r} in {project_root / locale_path}"
                )

    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--project-root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="repository root containing src/ and locales/",
    )
    args = parser.parse_args()

    errors = check_i18n_keys(args.project_root.resolve())
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    print("i18n keys: all literal Rust translation keys exist in every locale")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
