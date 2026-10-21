# Notices and attributions

PQ Studio is an independent graphical interface for the USGS geochemical
program PHREEQC. **It is not affiliated with, endorsed by, or supported by the
U.S. Geological Survey.** PHREEQC and PhreeqcI are named here only to describe
what this program works with and what it was modelled on.

## 1. Relationship to the original

**You do not have the original distribution.** PQ Studio is not
PHREEQC Interactive (PhreeqcI) for Windows, and it is not the original USGS
PHREEQC distribution. Please don't report problems with it to the USGS.

**The user interface is a new program.** It was written from scratch in Rust
(egui/eframe) and contains no source code from PhreeqcI or PHREEQC. It
follows the PhreeqcI user interface: the set of input screens, their fields
and groupings, and the keyword-tree workflow. That layout was derived
from the dialog and menu resource templates of PhreeqcI 3.8.6 for Windows, and
from the keyword documentation in Parkhurst and Appelo (2013). The visual
design (dark theme, plotting) is new.

**All calculations are done by the unmodified USGS PHREEQC engine.** The
interface writes a PHREEQC input file and runs the `phreeqc` program as a
separate process; it never reimplements any chemistry.

- **Ready-made packages** (the Linux `.tar.gz`, the macOS `.dmg` and the
  Windows `.zip`): each contains the `phreeqc` program version 3.9.0-17591
  and its database files, built from the official source release
  (`phreeqc-3.9.0-17591.tar.gz`, SHA-256
  `fda26290d96f6785e440c05217bf4b9fa9cd1efbf2fee155591e05b30165c6cd`) and
  shipped with the USGS notice below and upstream's own `NOTICE` file
  (`PHREEQC_NOTICE.txt`). The engine's own start-up banner in this release
  reads "PHREEQC_3.8.9"; that is how upstream labels it, not a different
  program.
- **Built from source on Linux** (`packaging/linux/install.sh` in the
  source repository): the interface runs whatever `phreeqc` the user has
  installed. Nothing from PHREEQC is shipped.
- **The source repository** contains no PHREEQC code or data. The package
  build scripts download the official source release and check its
  checksum.

**Extent of modifications to PHREEQC:**

- Source code: **none.** No PHREEQC source file is modified.
- Database files (`*.dat`): **none.** They are bundled byte-for-byte.
- Build configuration only, using upstream's CMake build in Release mode
  with the optional GNU MP library disabled (upstream's default), so the
  inverse-modelling `-multiple_precision` option is unavailable in the
  bundled engine. Per platform:
  - Linux: x86_64, with the C++ runtime linked statically
    (`-static-libstdc++ -static-libgcc`) so it runs on more distributions.
  - macOS: a universal (arm64 + x86_64) binary for macOS 11 and later.
  - Windows: x64, built with Microsoft Visual C++ with the C runtime linked
    statically (`/MT`), so no Visual C++ Redistributable is needed.
- Modifications made by: Espen, October 2026.

**Where to get the original:**

- PHREEQC (USGS software page, including official downloads):
  https://www.usgs.gov/software/phreeqc-version-3
- PHREEQC source releases: https://github.com/phreeqc-dev/phreeqc3/releases
- PHREEQC version 3 documentation: https://pubs.usgs.gov/tm/06/a43/

**Acknowledgement.** PHREEQC was written by David L. Parkhurst (USGS) and
C.A.J. (Tony) Appelo. The original PhreeqcI interface was written by Scott R.
Charlton and David L. Parkhurst (USGS). Please cite the original work in
publications that use results produced with this program:

- Parkhurst, D.L., and Appelo, C.A.J., 2013, Description of input and examples
  for PHREEQC version 3--A computer program for speciation, batch-reaction,
  one-dimensional transport, and inverse geochemical calculations: U.S.
  Geological Survey Techniques and Methods, book 6, chap. A43, 497 p.
- Charlton, S.R., and Parkhurst, D.L., 2002, PhreeqcI--A graphical user
  interface to the geochemical model PHREEQC: U.S. Geological Survey Fact Sheet
  FS-031-02, 2 p.

## 2. USGS PHREEQC User Rights Notice

The following notice accompanies PHREEQC and PhreeqcI and is reproduced here
verbatim, as its terms require.

```
                            User Rights Notice

This software and related material (data and (or) documentation),
contained in or furnished in connection with PHREEQC, are made available
by the U.S. Geological Survey (USGS) to be used in the public interest
and in the advancement of science.  You may, without any fee or cost,
use, copy, modify, or distribute this software, and any derivative works
thereof, and its supporting documentation, subject to the following
restrictions and understandings.

If you distribute copies or modifications of the software and related
material, make sure the recipients receive a copy of this notice and
receive or can get a copy of the original distribution.  If the software
and (or) related material are modified and distributed, it must be
made clear that the recipients do not have the original and they must
be informed of the extent of the modifications.  For example, modified
files must include a prominent notice stating the modifications made, the
author of the modifications, and the date the modifications were made.
This restriction is necessary to guard against problems introduced in the
software by others, reflecting negatively on the reputation of the USGS.

The software is public property and you therefore have the right to
the source code, if desired.

You may charge fees for distribution, warranties, and services provided
in connection with the software or derivative works thereof.  The name
USGS can be used in any advertising or publicity to endorse or promote
any products or commercial entity using this software if specific
written permission is obtained from the USGS.

The user agrees to appropriately acknowledge the authors and the USGS
in publications that result from the use of this software or in products
that include this software in whole or in part.

Although this software program has been used by the U.S. Geological
Survey (USGS), no warranty, expressed or implied, is made by the USGS
or the U.S. Government as to the accuracy and functioning of the
program and related program material nor shall the fact of distribution
constitute any such warranty, and no responsibility is assumed by the
USGS in connection therewith. 

The authors, the USGS, and the United States Government are not
obligated to provide the user with any support, consulting, training or
assistance of any kind with regard to the use, operation, and
performance of this software nor to provide the user with any updates,
revisions, new versions, or "bug fixes."

The user assumes all risk for any damages whatsoever resulting from
loss of use, data, or profits arising in connection with the access,
use, quality, or performance of this software.

```

## 3. Other components

- **Third-party Rust crates** compiled into the interface: see
  `THIRD_PARTY_LICENSES.md` (generated with `cargo about`).
- **DejaVu Sans fonts** (bundled for chart text): see `FONT_LICENSE.txt`.
- **This program's own code and icon:** MIT licence, see `LICENSE`.
