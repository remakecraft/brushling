# Brushling

A single-document raster paint program in Rust and egui, rewritten from scratch by [Remakecraft](https://github.com/remakecraft/remakecraft) from a 1998-era paint program's observed behaviour. No code from the original or from any known remake was used: the builders worked from a written spec and tests behind a logged wall, and known reimplementations were on a blocklist.

## Parity

Measured on 2026-10-13 against the conformance suite Remakecraft wrote before any code. About a third of the tests are sealed: the builders never had them, and only the harness ran them.

| Layer | Visible | Sealed | Overall |
|---|---|---|---|
| Behaviour | 181/182 | 84/84 | 99.6% |
| Pixel | 61/61 | 33/33 | 100% |
| File format | 36/36 | 18/18 | 100% |
| UI | 38/38 | 19/19 | 100% |
| **Total** | **316/317** | **154/154** | **99.7%** |

60 features (47 must, 10 should, 3 could), 2 h 56 min, $41.08 in tokens. The one failure, `tool.polygon`'s `click_first_vertex_closes`, is a spec defect: the test expects a pixel that is not on the triangle. Parity measures the spec, not the original, so a behaviour nobody wrote down is not tested.

## What is here

| Path | What |
|---|---|
| `src/` | The app as a library: `harness.rs` is the public `App` API, behaviour lives under `features/`. See [`src/README.md`](src/README.md) |
| `spec/` | The behaviour spec the builders worked from |
| `tests/visible/` | The visible conformance tests. The sealed tests and the evidence about the original are not published |
| `remake.json` | The run's settings, including the blocklist |

The app is a library with a test harness; there is no window entry point yet. Build it with:

```bash
cargo build --all-targets
```

## Not affiliated

Microsoft, Windows and Microsoft Paint are trademarks of the Microsoft group of companies. Brushling is an independent project, not affiliated with, endorsed by, or sponsored by Microsoft.

## Licence

[MIT](LICENSE).
