# 7. Accessibility and translation

## 7.1 Required for every component

A component is not complete until each item has a story or test.

- [ ] **Keyboard.** Every control works from the keyboard. Behaviour follows the suite's own apps first, WAI-ARIA authoring patterns where no app has a convention.
- [ ] **Visible focus.** A keyboard-only focus ring, always visible.
- [ ] **AccessKit.** Roles, names and values on every widget (button, spin button, slider, text input, combo box, list option, radio, dialog, menu item, tab, tree item).
- [ ] **RTL.** Chrome mirrors. Sliders, number fields and key chords never mirror.
- [ ] **Reduced motion.** Animations resolve to zero.
- [ ] **Translation.** Library words are rows in the `craft-i18n` catalog. App labels win. A missing string shows in English.
- [ ] **Edit lifecycle.** Value controls report begin, update, commit, cancel and set. Esc restores the value held before the gesture. A drag or typing run is one undo step.

Keyboard conventions:

- Tab visits fields only in dialogs and every control elsewhere.
- A swatch palette and a guide set are one Tab stop with arrow keys inside. A tab strip is one Tab stop.
- Ctrl+Tab focuses a dialog's title row.
- Cmd or Ctrl+] and [ move the focused row in every reorderable list. F2 renames. Enter runs the row's default action.

## 7.2 Translation rules

- Library words (button labels, dialog text, error text) are rows in the shared catalog (`common.tsv`). Apps do not translate library words themselves.
- The app installs its translator through the host once per language change.
- Lookup order: the language's row, the English row, the built-in English.
- A translation with a missing placeholder is ignored.
- `craft::words::all()` lists the word tables of every enabled crate for catalog extraction.
- No test requires a translation to exist.

## 7.3 Goals beyond the required list

These guide design where no rule exists. They are not acceptance criteria.

- High contrast is the accessible option and meets the standard ([5.7](5-foundations.md#57-appearance-and-contrast)). Standard contrast follows each theme's design.
- Where no app has a keyboard or pointer convention, follow the WAI-ARIA authoring practices as closely as the component allows.
- Size and Text size let users enlarge the interface without changing layout.
