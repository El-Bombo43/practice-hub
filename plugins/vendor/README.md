`nih_plug_egui/` is a copy of nih-plug's egui adapter (ISC licence) at the pinned commit with two changes:
`Cargo.toml` points at the pinned git rev instead of a relative path, and `EguiState::set_requested_size` is `pub`
so the Contour size presets (S/M/L) can resize the window.
