from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.i18n_key_check import (
    check_i18n_keys,
    extract_translation_references,
    parse_locale,
)


PROJECT_ROOT = Path(__file__).resolve().parent.parent


class LocaleParserTests(unittest.TestCase):
    def test_collects_nested_leaf_keys(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            locale = Path(temporary) / "en.yml"
            locale.write_text(
                'common:\n  save: "save"\nstatus:\n  blocked: "blocked"\n',
                encoding="utf-8",
            )

            keys, errors = parse_locale(locale)

        self.assertEqual(keys, {"common.save", "status.blocked"})
        self.assertEqual(errors, [])

    def test_reports_duplicate_key_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            locale = Path(temporary) / "en.yml"
            locale.write_text(
                'status:\n  done: "done"\n  done: "finished"\n',
                encoding="utf-8",
            )

            _, errors = parse_locale(locale)

        self.assertEqual(len(errors), 1)
        self.assertIn("duplicated locale key 'status.done'", errors[0])


class RustReferenceTests(unittest.TestCase):
    def test_extracts_multiline_literal_calls_and_ignores_other_macros(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "ui.rs"
            source.write_text(
                'let one = t!("status.done");\n'
                'let two = t!(\n    "mobile.tab_status", name = name\n);\n'
                'let unrelated = format!("status.idle");\n',
                encoding="utf-8",
            )

            references, errors = extract_translation_references(source)

        self.assertEqual(
            [reference.key for reference in references],
            ["status.done", "mobile.tab_status"],
        )
        self.assertEqual(errors, [])

    def test_rejects_dynamic_keys_that_the_gate_cannot_verify(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "ui.rs"
            source.write_text("let text = t!(key);\n", encoding="utf-8")

            _, errors = extract_translation_references(source)

        self.assertEqual(len(errors), 1)
        self.assertIn("plain string literal", errors[0])


class I18nKeyCheckTests(unittest.TestCase):
    def make_project(self, root: Path, rust_key: str = "status.blocked") -> None:
        (root / "src").mkdir()
        (root / "locales").mkdir()
        (root / "src" / "ui.rs").write_text(
            f'let label = t!("{rust_key}");\n', encoding="utf-8"
        )
        (root / "locales" / "en.yml").write_text(
            'status:\n  blocked: "blocked"\n', encoding="utf-8"
        )
        (root / "locales" / "zh.yml").write_text(
            'status:\n  blocked: "已阻塞"\n', encoding="utf-8"
        )

    def test_matching_catalogs_and_source_pass(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_project(root)

            errors = check_i18n_keys(root)

        self.assertEqual(errors, [])

    def test_wrong_namespace_is_reported_for_both_locales(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_project(root, rust_key="common.blocked")

            errors = check_i18n_keys(root)

        self.assertEqual(len(errors), 2)
        self.assertTrue(all("'common.blocked'" in error for error in errors))

    def test_locale_catalog_drift_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_project(root)
            (root / "locales" / "zh.yml").write_text(
                'status:\n  blocked: "已阻塞"\n  done: "已完成"\n', encoding="utf-8"
            )

            errors = check_i18n_keys(root)

        self.assertEqual(len(errors), 1)
        self.assertIn("extra locale key 'status.done'", errors[0])

    def test_repository_translation_keys_are_complete(self) -> None:
        self.assertEqual(check_i18n_keys(PROJECT_ROOT), [])


if __name__ == "__main__":
    unittest.main()
