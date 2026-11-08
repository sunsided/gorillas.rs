# Fonts

| File | Contents |
|------|----------|
| `vga8x14.bin` | 256 glyphs, 8x14: the character set the VGA BIOS uses in 350-line modes (`SCREEN 9`) |
| `vga8x16.bin` | 256 glyphs, 8x16: the VGA text-mode character set |

Raw glyph rows, 14 or 16 bytes per character, most significant bit leftmost.

Both come unchanged from **fntcol16**, Joseph (Yossi) Gil's collection of EGA/VGA text-mode
fonts: `VGA-ROM.F14` and `VGA-ROM.F16`. Its notice, which says that the individual fonts are in
the public domain and asks that his name, addresses and the notice stay with the files, is
[`fntcol16-COPYLEFT.TXT`](fntcol16-COPYLEFT.TXT), copied verbatim. The original archive was
`ftp://ftp.simtel.net/pub/simtelnet/msdos/screen/fntcol16.zip`; a mirror lives at
<https://github.com/perillamint/dkbfnts/tree/95d549e1f1d36d1774cf36e18357f76764be61f9/latin/fntcol16>.

How they compare with DOSBox-X, the reference the port is checked against:

- `vga8x14.bin` is byte for byte the 8x14 font DOSBox-X's VGA BIOS uses (dumped through the
  `INT 43h` vector with `tools/dosbox/FONT14.BAS`), so `SCREEN 9` text matches pixel for pixel.
- `vga8x16.bin` differs from DOSBox-X's 8x16 in 19 glyphs (`0 M T V W X Y Z m v w x` and a few
  CP437 ones): fntcol16's versions use all eight columns, IBM's leave the eighth empty. Text-mode
  screens therefore differ from DOSBox in those characters. An exact 8x16 exists under
  CC-BY-SA-4.0 (VileR's Ultimate Oldschool PC Font Pack, `Bm437_IBM_VGA_8x16`); the port uses the
  public-domain one to stay EUPL-1.2 only.
