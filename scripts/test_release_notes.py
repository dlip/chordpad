import unittest

from release_notes import notes_for_version


class ReleaseNotesTest(unittest.TestCase):
    def test_selects_only_the_tagged_version_and_preserves_markdown(self):
        changelog = """# Changelog

## Unreleased

- Future work.

## 1.2.0

### Added

- New chords.

### Fixed

- Controller reconnects.

## 1.1.0

- Older changes.
"""
        expected = "### Added\n\n- New chords.\n\n### Fixed\n\n- Controller reconnects.\n"
        self.assertEqual(notes_for_version(changelog, "v1.2.0"), expected)
        self.assertEqual(notes_for_version(changelog, "1.2.0"), expected)

    def test_accepts_a_version_heading_with_a_v_prefix(self):
        self.assertEqual(notes_for_version("## v1.0.0\n\n- First release.", "v1.0.0"), "- First release.\n")

    def test_does_not_confuse_similar_versions(self):
        changelog = "## 1.0.1\n- Patch.\n## 1.0.0\n- Initial.\n"
        self.assertEqual(notes_for_version(changelog, "v1.0.0"), "- Initial.\n")

    def test_missing_empty_and_duplicate_entries_fail(self):
        for changelog in ["## Unreleased\n- Later.\n", "## 1.0.0\n\n## 0.9.0\n- Old.\n",
                          "## 1.0.0\n- First.\n## v1.0.0\n- Duplicate.\n"]:
            with self.subTest(changelog=changelog), self.assertRaises(ValueError):
                notes_for_version(changelog, "v1.0.0")

    def test_unreleased_cannot_be_published(self):
        for tag in ["", "v", "Unreleased", "unreleased", "vUnreleased"]:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                notes_for_version("## Unreleased\n- Future.\n", tag)

    def test_headings_inside_fenced_examples_are_preserved(self):
        for fence in ["```markdown", "~~~markdown", "````markdown"]:
            closing = fence.removesuffix("markdown")
            inner = "\n```" if closing != "```" else ""
            body = f"- Example:\n\n{fence}\n## 2.0.0{inner}\n{closing}\n\n- Still this version."
            changelog = f"## 1.0.0\n\n{body}\n\n## 0.9.0\n- Older.\n"
            self.assertEqual(notes_for_version(changelog, "v1.0.0"), body + "\n")


if __name__ == "__main__":
    unittest.main()
