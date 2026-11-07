# Fonts

These files are **not** under the project's EUPL-1.2 license.

| File | Contents |
|------|----------|
| `vga8x14.bin` | 256 glyphs, 8x14, the character set the VGA BIOS uses in 350-line modes (`SCREEN 9`) |
| `vga8x16.bin` | 256 glyphs, 8x16, the VGA text-mode character set |

Both were dumped from the BIOS of DOSBox-X 2024.03.01 through the `INT 43h` vector, with
`tools/dosbox/FONT14.BAS` and `tools/dosbox/FONT16.BAS`. DOSBox-X ships this font data as part
of its source code:

- Copyright: 2002-2021 The DOSBox Team, 2011-2024 The DOSBox-X Team
- License: GPL-2.0-or-later (<https://www.gnu.org/licenses/old-licenses/gpl-2.0.html>)

The glyph designs follow the character sets of IBM's EGA/VGA ROMs.

The native binary and the WebAssembly build embed these files (`include_bytes!`), so they
contain GPL-2.0-or-later data.
