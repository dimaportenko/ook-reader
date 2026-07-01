# Sample EPUB fixture

`pg1661-adventures-of-sherlock-holmes.epub` is the app's bundled sample book, referenced by
`crate::BOOK` in `src/main.rs`. It's the only fixture the test suite and `dx serve` run against.

## Source

[Project Gutenberg ebook #1661](https://www.gutenberg.org/ebooks/1661), *The Adventures of
Sherlock Holmes* by Arthur Conan Doyle.

## Licence

Public domain in the US under Project Gutenberg's terms — safe to commit and redistribute. This
is what satisfies the phase's "DRM-free EPUBs only" constraint.

## Why this book

It exercises the whole render path in one file: a cover image, the book's own CSS, inline images,
a real table of contents with in-book links, and a 15-document spine. The render/paging/link tests
all have something real to bite on.

## Invariants the tests depend on

- Spine length is 15.
- "A Scandal in Bohemia" is spine index 2 and carries the `#chap01` fragment.
- The cover is a JPEG.

Changing the file breaks these tests on purpose.

## `book/unzipped/`

Gitignored scratch space — an unpacked copy of the epub for eyeballing its contents. Not the
fixture itself; the tracked `.epub` above is.
