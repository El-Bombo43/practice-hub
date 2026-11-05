`nih_plug_egui/` is a copy of nih-plug's egui adapter (ISC licence) at the pinned commit with two changes:
`Cargo.toml` points at the pinned git rev instead of a relative path, and `EguiState::set_requested_size` is `pub`
so the size presets (S/M/L, Headroom modes) can resize the window. `editor.rs` also stores the new size before calling `request_resize()` (upstream stores it afterwards, so the host resized to the old size).
