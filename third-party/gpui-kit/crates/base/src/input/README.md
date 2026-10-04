# Input module layout

The public Rust module remains `gpui_base::input`; the folders below organize
its implementation without adding public module-path segments.

- `base/` contains the shared text-editing engine and foundational behavior:
  state, layout, cursor, selection, movement, touch selection, masking, native
  integration, and painting.
- `input/` contains the single-line `Input` element and `InputState` facade.
- `textarea/` contains the multi-line `Textarea` element and `TextareaState`
  facade.
- `editor/` contains the `Editor` element and `EditorState` facade together
  with display mapping, highlighting, search, diagnostics, decorations,
  indentation, and LSP integration.

`mod.rs` is the external seam. Keep public re-exports there so reorganizing the
implementation does not change callers' imports.

The root uses explicit `#[path]` declarations so this organization remains an
implementation detail and existing internal module relationships stay intact.

## Local raw-input admission

Maka adds `InputState::filter_inserted_text` to this pinned upstream source.
The original editing state tests complete inserted text before normalization,
clipboard distribution, composition changes or undo-history mutation. A rejection
preserves the original text, selection and history; final-value validation remains
independent. The hook is restricted to single-line state and is unset by default.
Maka uses it to reject control characters rather than silently alter credentials.
