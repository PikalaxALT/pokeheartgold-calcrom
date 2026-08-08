## Purpose

This utility reports on progress for a Nintendo DS decompilation project. It was designed for [Pokémon HeartGold/SoulSilver](https://github.com/pret/pokeheartgold) but is generic enough to apply to any project using a NitroSDK-like build environment.

The output looks something like this:

```
Analysis of heartgold.us binary:
  3918800 total bytes of code
    874908 bytes of code in src (22.33%)
    3043892 bytes of code in asm (77.67%)

  484294 total bytes of data
    103542 bytes of data in src (21.38%)
    380752 bytes of data in asm (78.62%)

  570545 total pointers
    568620 properly-linked pointers (99.66%)
    1925 hard-coded pointers (0.34%)

Analysis of soulsilver.us binary:
  3918794 total bytes of code
    874898 bytes of code in src (22.33%)
    3043896 bytes of code in asm (77.67%)

  484294 total bytes of data
    103542 bytes of data in src (21.38%)
    380752 bytes of data in asm (78.62%)

  570546 total pointers
    568620 properly-linked pointers (99.66%)
    1926 hard-coded pointers (0.34%)

Analysis of ichneumon_sub binary:
  158412 total bytes of code
    11812 bytes of code in src (7.46%)
    146600 bytes of code in asm (92.54%)

  2602 total bytes of data
    118 bytes of data in src (4.53%)
    2484 bytes of data in asm (95.47%)

  4178 total pointers
    4162 properly-linked pointers (99.62%)
    16 hard-coded pointers (0.38%)
```

All sizes are with respect to the game as it is loaded into memory, as opposed to how it is laid out in the final compiled ROM which may be compressed. Whether a byte is classified as "code" or "data" depends on name of the the ELF section it lies in. Sections named `.text`, `.init`, `.itcm`, `.sinit`, and `.wram` are classified as code, whereas sections named `.data`, `.rodata`, `.sdata`, `.dtcm`, `.exception`, amd `.version` are classified as data. The C vs ASM classification is based on a mapping of the source tree.

A hardcoded pointer is defined as any aligned 32-bit value that lies within a potentially-loaded program segment. When this number reaches zero, we can confidently declare the ROM "shiftable", that is, we can introduce a change that shifts the addresses of all following symbols and still produce a valid program. Since there are several legitimite reasons why a 4-byte string can appear as a pointer without actually being intended as one, we expect about a thousand or two false detections by this heuristic. Thus this number will never truly reach zero. You can manually inspect the detected hardcoded pointers by running the dev/debug configuration.

## Usage

```bash
cargo run --release -- \
  -d path/to/pokeheartgold \
  -9 "" \
  -7 sub \
  heartgold.us soulsilver.us
```

## Version history

### `0.1.2` - 2026-08-08

Fix mishandling of filenames with dots in the middle

### `0.1.1` - 2026-08-07

Fix issues and panics when running in GitHub Actions

### `0.1.0` - 2026-07-31

Initial release
