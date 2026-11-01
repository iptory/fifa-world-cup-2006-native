# Third-party redistribution texts

Staged exact upstream texts. Verification: fetched from the official
upstream repository at the pinned revision, compared byte-for-byte with
the vendored/cached copy where one exists, and recorded with SHA-256.

## dxc — DirectXShaderCompiler API headers (Windows D3D12 path)

- Upstream: https://github.com/microsoft/DirectXShaderCompiler
- Vendored here from SDK `thirdparty/dxc/README.md`, commit
  `6b6f40200bea5ed99367513f53f6a28e52fd3d0e`
- File: `dxc/LICENSE.TXT`
- Source URL:
  https://raw.githubusercontent.com/microsoft/DirectXShaderCompiler/6b6f40200bea5ed99367513f53f6a28e52fd3d0e/LICENSE.TXT
- SHA-256: `721cb11de618fcf9bbb7d25a389207bf2227357e6694bc326ab32a6699f9b951`
- Bytes: 3281, lines: 71
- License: University of Illinois/NCSA Open Source License
  ("LLVM Release License" header). This is the short-form license text
  from that era of the repository; it is what `LICENSE.TXT` contained at
  the pinned commit. The current `main` branch carries a longer
  Apache-2.0-with-LLVM-exception text and is NOT used here.
- Coverage: the license reference in both vendored headers
  (`dxcapi.h`: "distributed under the University of Illinois Open Source
  License. See LICENSE.TXT"; `DxbcConverter.h`: identical wording).
- Verified: upstream `dxcapi.h` (30008 bytes) and `DxbcConverter.h`
  (2510 bytes) at the pinned commit are byte-identical to the SDK
  vendored copies.

## disruptorplus — timer-queue headers (rexcore, all desktop builds)

- Upstream: https://github.com/lewissbaker/disruptorplus
- Vendored SDK path: `thirdparty/disruptorplus/include/disruptorplus/`
  (11 headers, no LICENSE file vendored, no copyright headers in files).
- File: `disruptorplus/LICENSE.txt`
- Source URL:
  https://raw.githubusercontent.com/lewissbaker/disruptorplus/master/LICENSE.txt
- GitHub reports SPDX `MIT` for this repository.
- SHA-256: `a56f05c0dbd9489e928fd3ca22712f90b310fe749ceef47059e895f6a235100c`
- Bytes: 1077, lines: 21
- License: MIT License (MIT), copyright (c) 2013 Lewis Baker.
- Upstream HEAD: `748e354476e0bf86dfdbb55f1d925222d7ee631b`
  (2023-01-27, merge of saltzm/master; latest commit touching headers is
  `221ec17e46a6` 2022-09-23). `LICENSE.txt` itself dates to
  `43392bb711fc` (2013-12-20, "in preparation for open-source release").
- Verified: no pinned submodule/commit for this directory is recorded in
  the SDK (it is committed directly, not a submodule), so the license is
  taken from upstream `master`. Two headers are byte-identical to master
  (`sequence.hpp`, `config.hpp`); six differ only in `#include` style
  (`"sequence.hpp"` vs `<disruptorplus/sequence.hpp>`) or trailing blank
  lines; three carry small SDK-side adaptations:
  `multi_threaded_claim_strategy.hpp` (one `size_t` cast, one 4-arg
  `wait_until_published` overload call used by `timer_queue.cpp`),
  `spin_wait.hpp` (x86-arch `_mm_pause` branch), `sequence_barrier_group.hpp`
  (trailing whitespace only). The MIT text covers the upstream work; the
  SDK-side tweaks are project adaptations, not a different license.

## egui family v0.33.3, Rust Windows setup GUI

- Upstream: https://github.com/emilk/egui, tag `0.33.3`
  (commit `44cdd653e2317d300fb8a6c9c36b03f23991e803`, 2025-12-11;
  matches `.cargo_vcs_info.json` `sha1` in every cached package).
- Files:
  - `egui-0.33.3/LICENSE-MIT` — SHA-256
    `95ca92f5f8ea5231f1580b3a2a799e8260af3114b900e1def5355a7f44bcf60c`
    (1092 bytes, 26 lines)
  - `egui-0.33.3/LICENSE-APACHE` — SHA-256
    `8173d5c29b4f956d532781d2b86e4e30f83e6b7878dce18c919451d6ba707c90`
    (10850 bytes, 202 lines)
  - `egui-0.33.3/epaint_default_fonts-fonts-Hack-Regular.txt` — SHA-256
    `47c0cccbeec7e8614548cc485588b28149e7874188df5f41b36efebcee285c87`
    (3734 bytes; MIT + Bitstream Vera notices for the Hack typeface)
  - `egui-0.33.3/epaint_default_fonts-fonts-OFL.txt` — SHA-256
    `6a73f9541c2de74158c0e7cf6b0a58ef774f5a780bf191f2d7ec9cc53efe2bf2`
    (4301 bytes; SIL Open Font License 1.1, covers NotoEmoji-Regular)
  - `egui-0.33.3/epaint_default_fonts-fonts-UFL.txt` — SHA-256
    `2f0015108d68627bd788d313f529c21ff4da2c2c42a5e1f3883acc83480f9002`
    (4673 bytes; Ubuntu Font Licence 1.0, covers Ubuntu-Light)
  - `egui-0.33.3/epaint_default_fonts-fonts-emoji-icon-font-mit-license.txt`
    — SHA-256
    `b9d2c1d909aa149996fd4c91dcb92b2362a04431640c1d200959da94caf8cde1`
    (1069 bytes; MIT for the emoji-icon font)
- Source URLs (all at tag `0.33.3`):
  - https://raw.githubusercontent.com/emilk/egui/0.33.3/LICENSE-MIT
  - https://raw.githubusercontent.com/emilk/egui/0.33.3/LICENSE-APACHE
  - https://raw.githubusercontent.com/emilk/egui/0.33.3/crates/epaint_default_fonts/fonts/Hack-Regular.txt
  - https://raw.githubusercontent.com/emilk/egui/0.33.3/crates/epaint_default_fonts/fonts/OFL.txt
  - https://raw.githubusercontent.com/emilk/egui/0.33.3/crates/epaint_default_fonts/fonts/UFL.txt
  - https://raw.githubusercontent.com/emilk/egui/0.33.3/crates/epaint_default_fonts/fonts/emoji-icon-font-mit-license.txt
- Coverage: all nine `0.33.3` crates in `Cargo.lock` —
  `ecolor`, `eframe`, `egui`, `egui-wgpu`, `egui-winit`, `egui_glow`,
  `emath`, `epaint`, `epaint_default_fonts` — are dual-licensed
  `MIT OR Apache-2.0` (workspace license in the upstream root
  `Cargo.toml`; `epaint_default_fonts` additionally
  `AND OFL-1.0 AND Ubuntu-font-1.0`). The cached `.crate` packages
  contain no license text files at top level (only `README.md`,
  manifests, sources; font `.txt` files live one level down under
  `fonts/`). All four cached font `.txt` files are byte-identical to
  the upstream tag. The MIT/Apache texts are the upstream repo-root
  license files at the same tag.
- Note: `egui-wgpu` is in the lockfile but the importer builds eframe
  with `glow` (not `wgpu`); the texts above cover the family either way.

## Independent JPEG Group (IJG) — no new file staged (attribution, not a file)

- The FFmpeg sources compiled in (`libavcodec/jfdctfst.c`,
  `libavcodec/jfdctint_template.c`, `libavcodec/jrevdct.c`) carry the IJG
  notice in their file headers: binaries must state in accompanying
  documentation that "this software is based in part on the work of the
  Independent JPEG Group", and any changes to those three files must be
  indicated (this tree builds them unmodified). IJG reference: the
  license text is published as SPDX `IJG`
  (https://spdx.org/licenses/IJG.html) and by the group at
  https://www.ijg.org/ (current release-10 package; same conditions).
- Action: the required sentence is added to the repository README (root
  `README.md`), which is the distribution's accompanying documentation.
  No separate license file is staged for IJG because the obligation is a
  documentation credit, satisfied by that README line.

## Distribution boundary

The downloadable setup contains authored Rust executables and project source. It
does not contain the game executable, generated game source, extracted disc data,
or compiled ReXGlue/FFmpeg/libmspack libraries. The setup creates these locally.

Rust dependency license texts are inventoried from the locked Windows dependency
closure and included in `THIRD_PARTY_LICENSES.txt` in each setup release. Native
patches retain the upstream BSD-3-Clause terms in `rexglue/LICENSE`. Original
game material and trademarks are not covered by the project's MIT license.

This inventory records provenance and distribution obligations. It is not a
legal opinion or a guarantee against copyright, trademark or circumvention claims.
