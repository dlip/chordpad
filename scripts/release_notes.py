#!/usr/bin/env python3
"""Extract one version's release notes from CHANGELOG.md."""

import argparse
from pathlib import Path
import re


def notes_for_version(changelog, tag):
    version = tag.removeprefix("v")
    if not version or version.casefold() == "unreleased":
        raise ValueError("Release notes require a version number")

    sections = []
    current = None
    fence = None
    for line in changelog.splitlines(keepends=True):
        if fence:
            if re.fullmatch(rf" {{0,3}}{re.escape(fence[0])}{{{len(fence)},}}[ \t]*", line.rstrip("\r\n")):
                fence = None
        else:
            marker = re.match(r" {0,3}(`{3,}|~{3,})", line)
            heading = re.match(r"^##[ \t]+(.+?)\s*$", line)
            if marker:
                fence = marker.group(1)
            elif heading:
                current = [] if heading.group(1).removeprefix("v") == version else None
                if current is not None:
                    sections.append(current)
                continue
        if current is not None:
            current.append(line)

    if not sections:
        raise ValueError(f"No changelog entry for {tag}; add a '## {version}' heading")
    if len(sections) != 1:
        raise ValueError(f"Duplicate changelog entries for {tag}")
    notes = "".join(sections[0]).strip()
    if not notes:
        raise ValueError(f"Changelog entry for {tag} is empty")
    return notes + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag")
    parser.add_argument("--changelog", type=Path, default=Path("CHANGELOG.md"))
    args = parser.parse_args()
    try:
        notes = notes_for_version(args.changelog.read_text(encoding="utf-8"), args.tag)
    except (OSError, ValueError) as error:
        parser.error(str(error))
    print(notes, end="")


if __name__ == "__main__":
    main()
