#!/usr/bin/env python3
"""Prepare a reproducible Leipzig English web corpus for Chordpad."""

import argparse
import hashlib
import io
import json
from pathlib import Path
import tarfile
import unicodedata
import urllib.request


CORPUS = "eng_web_2014_10K"
URL = f"https://downloads.wortschatz-leipzig.de/corpora/{CORPUS}.tar.gz"
ARCHIVE_SHA256 = "510dfc6c74765002643d6f3115a0c7502b7e437a2389dea64f7c2cecea310d18"
PUNCTUATION = str.maketrans({
    "‘": "'", "’": "'", "“": '"', "”": '"',
    "–": "-", "—": "--", "…": "...", "−": "-",
})


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def prepare(archive_bytes):
    if sha256(archive_bytes) != ARCHIVE_SHA256:
        raise ValueError("Corpus archive checksum differs from the recorded source")
    with tarfile.open(fileobj=io.BytesIO(archive_bytes), mode="r:gz") as archive:
        member = archive.extractfile(f"{CORPUS}/{CORPUS}-sentences.txt")
        rows = member.read().decode("utf-8").split("\n")
    buckets = {"train": [], "held-out": []}
    seen = set()
    discarded = {"unsupported_characters": 0, "duplicate_or_empty": 0}
    source_sentences = 0
    normalized_sentences = 0
    for row in rows:
        if not row:
            continue
        source_sentences += 1
        sentence_id, text = row.rstrip("\r").split("\t", 1)
        if not sentence_id.isdecimal():
            raise ValueError("Invalid sentence ID in source corpus")
        normalized = unicodedata.normalize("NFKC", text.translate(PUNCTUATION)).strip()
        if any(not 32 <= ord(c) <= 126 for c in normalized):
            discarded["unsupported_characters"] += 1
            continue
        if not normalized or normalized in seen:
            discarded["duplicate_or_empty"] += 1
            continue
        seen.add(normalized)
        normalized_sentences += normalized != text
        bucket = "held-out" if int(sha256(normalized.encode("utf-8")), 16) % 10 == 0 else "train"
        buckets[bucket].append((sentence_id, normalized))
    if source_sentences != 10_000 or not all(buckets.values()):
        raise ValueError("Unexpected source size or empty corpus split")
    manifest = {
        "corpus": CORPUS,
        "provider": "Leipzig Corpora Collection, Leipzig University",
        "source_url": URL,
        "archive_sha256": ARCHIVE_SHA256,
        "source_sentences": source_sentences,
        "normalized_sentences": normalized_sentences,
        "discarded": discarded,
        "normalization": "Curly quotes, en/em dashes, ellipsis and minus mapped to ASCII; NFKC; trim surrounding whitespace; discard remaining non-printable/non-ASCII sentences; deduplicate",
        "split": "SHA256 of normalized sentence modulo 10: 0 held-out, 1-9 train; original source order retained",
        "boundaries": "One sentence per line; source document order is unavailable. Windows across newline boundaries are approximations between unrelated sentences.",
        "terms_url": "https://wortschatz.uni-leipzig.de/en/usage",
        "format_url": "https://www.wortschatz.uni-leipzig.de/documents/Format_Download_File-eng.pdf",
        "splits": {},
    }
    files = {}
    for name, sentences in buckets.items():
        text = "".join(sentence + "\n" for _, sentence in sentences)
        files[f"{name}.txt"] = text
        manifest["splits"][name] = {
            "sentences": len(sentences),
            "characters": len(text),
            "words": len(text.split()),
            "sha256": sha256(text.encode("utf-8")),
            "source_sentence_ids": [sentence_id for sentence_id, _ in sentences],
        }
    return files, manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, help="Use an existing source archive instead of downloading")
    parser.add_argument("--output-dir", type=Path, default=Path("corpora/english-web"))
    args = parser.parse_args()
    if args.archive:
        data = args.archive.read_bytes()
    else:
        with urllib.request.urlopen(URL, timeout=60) as response:
            data = response.read()
    files, manifest = prepare(data)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    for name, text in files.items():
        (args.output_dir / name).write_text(text, encoding="utf-8")
        summary = manifest["splits"][name.removesuffix(".txt")]
        print(f"{args.output_dir / name}: {summary['sentences']:,} sentences, {summary['characters']:,} characters")
    (args.output_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
