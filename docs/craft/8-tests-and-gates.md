# 8. Tests and gates

## 8.1 Gates

| Gate | What it holds |
| --- | --- |
| Gallery | Stories and variants for every component. It covers every crate with a `ui` feature. Keep it in step with the features each app enables |
| Token dumps | Text files of resolved tokens per theme-mode variant, contrast (standard, high) and size (Compact, Comfortable), plus app sizes. Rewrite with `UPDATE_DUMPS=1` |
| Style dumps | One egui style file per theme-mode |
| Component dumps | One file per component in `craft-ui`, `craft-color`, `craft-draw` and `craft-imaging` |
| Story registry | Every kernel story and its default args |
| Title-bar dumps | The title bar's rendering |
| Screenshots | Visual regression, screens, docking workflows, interface scale, font rendering, web fonts. |
| Conformance | An app's real appearance setup against the library's reference, across the case matrix. Fails on a wrong theme, size or text size, a changed `Style`, or an app's own zoom or font override. `allow(field, reason)` makes an exception |
| Theme check | `craft-theme check <file>`: contrast report, adjustments, invalid fields |
| Design lint | `craft-design-lint check` (syn) and `check-clippy` |

Text dumps of every token and component style are the regression gate.

## 8.2 Design lint

- Rules: `literal-size/*`, `raw-colour/const`, `style-mutation`, `tooltip/stock`, `clippy/inner-allow`.
- Exemption marker: `// design: allow(reason)`. Empty, invalid and unused markers are findings.
- Baselines and `--deny-unused` are supported.
- Exit 0 clean, 1 findings, 2 error.
