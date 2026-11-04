# Headroom

**One line:** a mastering meter for a friend's master bus. It shows loudness (LUFS) and clipping (true peak) on a scrolling waveform. It opens in a full view and collapses to a small compact box that stays open on a single screen while he tweaks everything else.

**Category:** analyser (audio passes through untouched)  **Channels:** stereo in/out (also mono)  **Sidechain:** no  **MIDI:** none

**Lives at:** `practice-hub/plugins/headroom/SPEC.md`. "Headroom" is a working name and can be renamed freely before first real use.

**Who uses it and where**
- **Primary user:** Trent's friend. Ableton Live 12 **Standard** on **macOS**, **one monitor**. He uses it on the master track while mastering.
- **Also tested by Trent:** Live 12 Standard on Windows.
- **Why a collapsible window:** Ableton never shows third-party plugin UIs inside the device chain. Only stock and Max for Live devices can sit there, and Standard has no Max for Live. So the goal is a window that never needs closing: full detail when he wants it, and a small box parked in a corner while he works on other tracks.
- **Licensing:** shared with one friend, which counts as distribution under GPL-3.0 (nih-plug's VST3 bindings). He gets the source too, through repo access or a source zip with the build. No sale.

**Shared conventions:** same as Contour/Chamber/Vox Strip/Punch/Snap: practice-hub palette with hybrid accents, Sora/IBM Plex Mono, standard knob/field interactions, [Must]/[Should].

---

## 1. Platforms and builds [Must]
- **macOS** VST3 as a universal binary (Apple Silicon and Intel), built on a GitHub Actions **macOS runner**. It must be **ad-hoc code-signed** (`codesign --sign -`), otherwise Apple Silicon won't load it. It is not notarised.
- **Windows** VST3 as with the other plugins.
- CI uploads both as separate artifacts. The Mac artifact is a zip containing `Headroom.vst3` and an `INSTALL-mac.txt` with these steps:
  1. Copy `Headroom.vst3` to `~/Library/Audio/Plug-Ins/VST3/`.
  2. In Terminal, run `xattr -dr com.apple.quarantine ~/Library/Audio/Plug-Ins/VST3/Headroom.vst3`. This clears the "downloaded from the internet" block. It's needed once per new build.
  3. In Ableton, go to Settings → Plug-Ins, turn on **Use VST3 Plug-In System Folders**, then **Rescan**.
  4. In the same Settings page, turn **Multiple Plug-In Windows** on and **Auto-Hide Plug-In Windows** off, so the strip stays open while he selects other tracks.
  5. Put Headroom **last** on the Master track, after the limiter.
- AU is out of scope. Ableton on Mac loads VST3 fine.

## 2. What it measures [Must]
Everything follows **ITU-R BS.1770-4 / EBU R128** (K-weighting, gating, and 4× oversampled true peak). The builder implements this from the published standard.

| Measure | Definition | Display |
|---|---|---|
| Momentary | 400 ms window, K-weighted | `-12.4 LUFS` |
| Short-term | 3 s window | `-11.9 LUFS` |
| Integrated | Gated (absolute −70 LUFS, relative −10 LU) since last reset | `-13.2 LUFS` |
| Loudness Range (LRA) | EBU Tech 3342 | `6.4 LU` |
| True peak | 4× oversampled, max since reset, per channel and overall | `+0.4 dBTP` |
| Sample peak | max absolute sample since reset | `-0.1 dBFS` |
| PLR | True peak max − Integrated | `13.6 dB` |
| Overs | count of events where true peak exceeds the **target ceiling** (one event = consecutive over-samples, merged if within 50 ms) | `7` |
| Clips | count of events where the sample peak ≥ 0 dBFS | `2` |

- **Audio passes through bit-identically.** It reports latency 0. Metering happens on a copy.
- **Measuring runs only while the transport plays** (default; a setting allows "always"). Integrated and LRA **auto-reset when playback starts from a stopped transport** (default on; a setting can turn it off). A **Reset** button resets everything.

## 3. Targets [Must]
A dropdown in the strip. Each entry sets the target loudness and the true-peak ceiling. These are the services' published normalisation levels **as of Nov 2026**, so keep them editable in code.

| Preset | Target | Ceiling |
|---|---|---|
| Spotify | −14 LUFS | −1 dBTP (−2 if integrated is louder than −14) |
| Apple Music | −16 LUFS | −1 dBTP |
| YouTube | −14 LUFS | −1 dBTP |
| Tidal / Amazon | −14 LUFS | −1 dBTP |
| Deezer | −15 LUFS | −1 dBTP |
| Custom | user value, −30 to −6 | user value, −3 to 0 |

- The **"Will be turned down by"** readout = Integrated − Target, when positive (`Spotify: −0.8 dB`). If negative, it shows `quieter than target`. Spotify can also turn quiet tracks up, but that's not shown in v1.
- Integrated is shown in the accent colour within ±1 LU of target, amber when 1–3 LU off, red beyond that. True peak shows red whenever it's above the ceiling.

## 4. Parameters
Headroom is an analyser, so nothing needs automation. **All settings are saved non-automatable state**, not host parameters, which keeps Ableton's automation list clean:
- target preset
- custom target and ceiling
- window mode and width
- history length
- auto-reset
- measure-when-stopped

---

## 5. Look

### Window modes [Must]
Mac sizes are in points, so Retina doubles them automatically. Windows scales with DPI.

| Mode | Size | Use |
|---|---|---|
| **Full** (default) | 960 × 560 | Opens here. The whole song, overs list, meters and stats. The friend asked for this as the starting view. |
| **Strip – Wide** | 1200 × 132 | Parked along the top or bottom of the screen |
| **Strip – Narrow** | 860 × 132 | Smaller screens |
| **Compact** | 480 × 170 | Collapsed: a small box parked in a corner, wave plus numbers, while he works on other tracks |

- The last mode used is saved with the set. A new instance opens in Full.
- Ableton remembers the window position.

### Window sizer [Must: buttons · Should: drag grip]
The friend's request: open full, and shrink it down to the compact box when he moves on to other tracks.
- **Collapse button [Must]** in Full's top bar (and in the Strips): one click goes to Compact. The **expand button** in Compact goes straight back to Full. These are the main controls.
- **Mode switcher [Must]** in Full's top bar: Compact / Strip / Full segmented buttons. Strip remembers Wide or Narrow (right-click to choose).
- **Drag grip [Should]** in the bottom-right corner of every mode: dragging it resizes the window and **snaps** to the nearest mode (Full ↔ Strip ↔ Compact) on release. There's no free, in-between size, so each layout stays a fixed design. The plugin asks the host to resize. If a host refuses, the buttons still work.

### Compact layout [Must]
```
┌──────────────────────────────────────────────┐
│ INT -13.2   TP +0.4   S-T -11.9   OVERS 7 ⟲  │  numbers row, colour-coded as in section 3
├──────────────────────────────────────────────┤
│ ▁▃▅▇█▇▅▃▂▃▅▇██▇▅  waveform + red overs        │  same two-layer display as the strip,
│ ─ ─ ─ ceiling ─ ─ ─   ── LUFS line ──         │  last 30 s
└──────────────────────────────────────────────┘
```
- The target preset and the "will be turned down by" readout move to a small target pill next to the overs counter, e.g. `Spotify ↓0.8`. Click it to open the target menu.
- LRA is shown only in the Strip and Full modes.

### Strip layout [Must]
```
┌──────────┬────────────────────────────────────────────────────────────┬──────────────┐
│ INT -13.2│  ▁▃▅▇█▇▅▃▂▃▅▇██▇▅  waveform (true-peak envelope)          │ [Spotify ▾]  │
│ S-T -11.9│  ─ ─ ─ ─ ─ ─ ─ ─ ─ ceiling line (red dashed) ─ ─ ─ ─ ─ ─  │ ↓ 0.8 dB     │
│ TP  +0.4 │  ─────── short-term LUFS line over a target band ───────   │ OVERS 7  ⟲  │
│ LRA 6.4  │  (last 10 / 30 / 60 s, scrolls right to left)             │ [≡ Full]     │
└──────────┴────────────────────────────────────────────────────────────┴──────────────┘
```
- **Left:** four big numbers in IBM Plex Mono: Integrated, Short-term, True peak and LRA. Each is colour-coded as in section 3.
- **Centre:** a scrolling display with two layers sharing the space:
  1. **Waveform:** a mirrored true-peak envelope (one bar per ~20 ms), grey `#5a5a60`. Any bar whose true peak exceeds the ceiling is drawn **red** `#ff6b6b` at full height, with a small red tick above it. The ceiling is a red dashed line.
  2. **Short-term LUFS line** on its own right-hand scale (−30 to 0 LUFS): white `#e8e8ea`, 1.5 px, over a translucent accent band at target ±1 LU. Where the line runs above the band, it turns amber.
  - History length is 10 / 30 / 60 s (right-click menu, default 30 s). Redraws at 60 fps.
- **Right:** target dropdown, the turned-down readout, overs counter, Reset (⟲), and the collapse/expand button.
- Background `#0a0a0b`, panels `#141416`, accent `#d4ff3a`, warnings amber `#ffb347` and red `#ff6b6b`.

### Full mode [Should]
- **Whole-song history:** the same two layers, compressed to fit everything since the last reset. It records up to 20 minutes, then the oldest data drops off.
- **Overs list:** each over with its **song position** (bar.beat from the host transport, plus mm:ss) and its peak value. For example: `Bar 57.3 · 2:14 · +0.4 dBTP`.
  - Clicking a row highlights that spot in the history. A plugin can't move Ableton's playhead, so the list tells him where to go.
- **Big meters:** momentary and short-term bars (−40 to 0 LUFS) and true-peak bars per channel (−24 to +3 dBTP). Peak hold 2 s, fall 20 dB/s.
- **Stats table:** Integrated, LRA, PLR, max momentary, max short-term, max true peak (L/R), sample peak, clips.
- **[Should] Correlation meter** (−1 to +1), to catch mono-compatibility problems.

### Interactions
- Click the overs counter: switches to Full mode and scrolls to the overs list.
- Double-click the Integrated value: Reset.
- Hover the waveform: shows the time, short-term LUFS and true peak at that moment.

---

## 6. Edge cases
- **Transport stopped:** the display freezes (unless measure-when-stopped is on). Values hold.
- **Silence:** Integrated stays at `—` until at least 400 ms passes the −70 LUFS gate.
- **Mono track:** treated per BS.1770 as a single channel. If a mono signal arrives on a stereo master, both channels count normally.
- **Sample-rate change:** reset all meters and resize the internal filters and oversamplers. Measurements are valid at 44.1–192 kHz.
- **Looping a section:** each loop pass keeps adding to Integrated. That's expected; he can Reset at the loop start.
- **Export/freeze in Ableton:** the plugin runs offline, faster than real time. Measurements stay correct, and the GUI simply catches up.
- **Placed before the limiter by mistake:** fine, just less useful. INSTALL-mac.txt says to put it last.

## 7. Acceptance checks
1. Audio output is bit-identical to input. Reported latency is 0.
2. Passes the **EBU Tech 3341** minimum-requirement test cases: momentary, short-term and integrated readings within ±0.1 LU (cases 1–10, synthesised as stereo sines and noise per the document).
3. True peak passes the EBU Tech 3341 true-peak cases within the allowed tolerance (+0.2 / −0.4 dB), including inter-sample overs that sample peak misses.
4. LRA matches the **EBU Tech 3342** test cases within ±1 LU.
5. Overs: a test file with 5 separated inter-sample overs above −1 dBTP reports exactly 5. Two overs 30 ms apart count as 1.
6. Bar.beat positions in the overs list match the host transport within one 16th note at 120 BPM.
7. Measurement is identical at 44.1, 48 and 96 kHz (within ±0.1 LU and ±0.2 dB TP).
8. CPU well under 3% of a core, stereo, 48 kHz. The GUI closed or minimised costs nothing extra.
9. Passes pluginval at strictness 5 on macOS and Windows. Settings recall exactly after reopening the set.
10. **Mac check (friend):** loads in Live 12 on his Mac after the install steps; the strip stays open while he selects other tracks and tweaks his limiter; numbers match his current meter within ±0.2 LU on the same song.

## 8. Out of scope (v1)
Moving Ableton's playhead · inline device-chain display (would need Max for Live) · Spotify turn-up logic · reference-track A/B · spectrum or EQ display · loudness correction (it never changes audio) · AU format · notarised installer.
