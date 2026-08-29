# Feature — Chrome Material

[← Milestone 3: Reader Enhancements](../README.md)

**Goal:** the reader's floating surfaces — popovers first, then the bars — read as *glass*:
they take their colour and light from the book page behind them, in the same material on
macOS, iOS, Android and Linux.

This is the app's **chrome**, not the book. [ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md)
governs the other direction — the reader-controlled CSS layered over the publisher's inside
the chapter `<iframe>`. Nothing here reaches into that iframe; the material sits on the
Dioxus side of the boundary and samples through it.

## Phases

| # | Phase | Outcome | Status |
|---|---|---|---|
| 10 | [Liquid glass chrome](phase-10-liquid-glass.md) | A `.glass` primitive on the surfaces that already float, honouring `prefers-reduced-transparency` | 🚧 in progress — Step 1 written, uncommitted |

## Why it earned a feature directory

It started as "can we get Liquid Glass buttons on iOS?" and the answer turned out to be a
design decision with a paper trail, not a CSS snippet — see the phase's *crux*. Two routes
were investigated and rejected on the record (Apple's private `-apple-visual-effect`, and
the WebGL glass libraries), and the surviving route has a **layout prerequisite** that the
iOS port deliberately created. That is a phase, not a TODO line.
