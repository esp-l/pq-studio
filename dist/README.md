# PQ Studio

A desktop application for building, running and plotting **PHREEQC**
geochemical models. PHREEQC is the U.S. Geological Survey's program for
speciation, batch-reaction, one-dimensional transport and inverse geochemical
calculations. PQ Studio is a graphical interface for it: you assemble an
input from forms, pick a thermodynamic database, run the unmodified USGS
`phreeqc` engine, then read the output and plot the results.

> **Not a USGS product.** PQ Studio is independent and is not affiliated
> with, endorsed by or supported by the U.S. Geological Survey. Its workflow
> is modelled on the USGS Windows program PHREEQC Interactive (PhreeqcI), and
> all calculations are done by the unmodified USGS `phreeqc` engine. Please
> don't send questions about PQ Studio to the USGS or the PHREEQC authors.
> See `NOTICE.md` for details and for where to get the original software.

## Using PQ Studio

1. **Start from an example or a new document.** *Help → Open Example* loads
   a ready-made input; *File → New* starts empty; *File → Open…* reads an
   existing PHREEQC input file (`.pqi`, `.phr`, `.txt`).
2. **Add blocks.** *Insert* adds a keyword block (Solution, Equilibrium
   Phases, Reaction, Kinetics, Transport, Selected Output and more). The tree
   on the left lists the blocks in run order; click one to edit it in the
   middle. Delete a block with its ✕ button or the Delete key.
3. **Fill in the forms.** Numeric fields are outlined in red when the text
   isn't a number, and Run tells you which fields to fix. Leaving a field
   blank uses PHREEQC's default.
4. **Pick a database.** The *Database* list next to Run shows the databases
   that came with the engine (`phreeqc.dat`, `wateq4f.dat`, `llnl.dat`,
   `pitzer.dat`, …). *Browse…* picks your own `.dat` file. The choice is
   remembered.
5. **Run.** Press **▶ Run** (Ctrl+R or F5; ⌘R on a Mac). The window stays
   responsive while the engine works, and **Cancel** stops it. A run is also
   stopped automatically after 30 minutes or if the engine uses more than
   4 GB of memory, which protects against runaway inputs.
6. **Read the output.** The full engine output appears at the bottom left.
   *Find* highlights every match and steps through them; *Copy* and *Save
   output as…* keep a copy. If the engine reports errors, the relevant lines
   are shown in red above the output.
7. **Plot.** If the input has a Selected Output block, the plot window opens
   after the run with the chart and every setting beside it: *Data* (X axis,
   Y series, simulations shown), *Series style* (colour, line type and width,
   marker shape and size, legend label, *Reset to default*), *Axes* (titles,
   ranges, log scales, text sizes, grid), *Legend* (eight positions inside
   the chart, *Outside right* or *Hidden*, font size, frame, background
   opacity) and *Export* (figure size and resolution). Changes show at once.
   Close the window to get back to the editor; the small chart in the output
   panel (or *⛶ Plot window*) opens it again. *Export plot…* saves a PNG
   that matches the screen, or an SVG.
8. **Save.** *File → Save* (Ctrl+S) or *Save As…* (Ctrl+Shift+S). The file
   name and an "unsaved" marker show at the top of the block tree, and PQ
   Studio asks before discarding unsaved changes. *File → Open Recent* lists
   recent files.

**Several simulations.** PHREEQC splits an input into simulations at each
END, and each simulation reacts one solution or mix: the one named by *USE*,
else a MIX defined there, else the first SOLUTION defined there. The tree
groups blocks by simulation and names each one after what it reacts
("Simulation 3 · NaCl · Calcite"). *Insert → End* starts a new simulation,
new blocks go right after the selected one, *Edit → Move block up/down*
(Alt+↑/↓) reorders, and right-clicking a simulation's heading deletes it.

To run several waters through several set-ups, use *Run → Simulations from
combinations…*: tick the solutions and mixes to run, and for each reactant
type (temperature, pressure, equilibrium phases, reaction, …) tick one or
more choices, including *none*. Every ticked water is run with every
combination, e.g. 4 brines × {none, CO2, Calcite, Calcite and CO2} = 16
simulations. PQ Studio writes them as ordinary `USE … / END` blocks you can
see, edit and save. Using the dialog again replaces them; ticking no water
removes them. A yellow note under Run warns when solutions are defined but
never reacted because a simulation falls back on PHREEQC's default.

**Results table and plot.** The Selected Output form chooses the columns:
standard ones (pH, temperature, ionic strength…), saturation indices
(`si_Calcite`), equilibrium-phase amounts and changes (`Calcite`,
`d_Calcite`: negative = dissolved, positive = precipitated), totals,
molalities, activities and gases. With several simulations the chart draws
one line per simulation, named after it; *Simulations shown* hides or shows
each one. Each Y column has its own style (by default a solid line with
circles in its own colour; a mineral's columns share it, so si_Calcite,
d_Calcite and Calcite match, and the common minerals always get the same
colour: Calcite blue, Aragonite orange, Dolomite aqua, Anhydrite yellow,
Gypsum pink, Magnesite green, Witherite violet, CO2(g) red, Barite brown,
Halite charcoal, Strontianite plum, Celestite navy, Siderite dark gold).
With saturation indices plotted, a dashed line marks SI = 0 (switch it off
under *Axes*). *Presets* saves a whole plot set-up (columns, axes, legend,
export settings) under a name for reuse in any project. *Export plotted
data…* saves just what the chart shows as an Excel workbook (one sheet with
everything plus one per simulation) or CSV, for plotting elsewhere. The colour button offers ready-made colours (basic,
Matplotlib, colour-blind safe, PQ Studio's own) above a full colour picker and keeps it in every plot and every project,
also after unticking and ticking it again; styles you change are saved at
once. With one column over several simulations each simulation has its own
style; with several columns over several simulations each simulation gets
its own marker shape. The plot settings are saved with the project in a small
file beside it (`run.pqi` → `run.pqstudio.json`), so they come back when the
project is opened again; the `.pqi` itself stays plain PHREEQC input.
*Export table…* saves the whole table as CSV with a `name` column
for each row's simulation, and *Copy table* puts it on the clipboard for
pasting into a spreadsheet.

**Database definitions.** *Insert → Database Definitions* adds PHASES, the
solution/exchange/surface species and master-species keywords, PITZER and SIT
blocks. They are edited as PHREEQC text, starting from a commented template
that shows the syntax.

**Keyboard shortcuts** (⌘ instead of Ctrl on a Mac): New Ctrl+N, Open Ctrl+O,
Save Ctrl+S, Save As Ctrl+Shift+S, Run Ctrl+R or F5, Quit Ctrl+Q, Delete
removes the selected block, Alt+↑/↓ moves it.

## Install

Ready-made packages for Windows, macOS and Linux are on the project's
GitHub **Releases** page. Each one includes the PHREEQC engine and its
databases, so there is nothing else to install.

### Windows (10 and 11, 64-bit)

1. Download `PQ-Studio-<version>-windows-x64.zip`.
2. Right-click the downloaded file, choose **Extract All…**, and extract it
   somewhere permanent, for example your **Documents** folder. (Don't run it
   from inside the zip.)
3. Open the extracted **PQ Studio** folder and double-click
   **PQ Studio.exe**.
4. The first time, Windows may show "Windows protected your PC", because the
   app isn't signed with a paid certificate. Click **More info**, then
   **Run anyway**. You only have to do this once.

Optional: right-click **PQ Studio.exe** → **Show more options** →
**Send to → Desktop (create shortcut)**, or **Pin to Start**.

To remove it, delete the **PQ Studio** folder (and, if you like, the
settings folders listed under *Where things are stored*).

### macOS (11 Big Sur or later, Apple silicon and Intel)

1. Download and open `PQ-Studio-<version>-macos-universal.dmg`.
2. Drag **PQ Studio** onto the **Applications** shortcut in the same window.
3. Eject the disk image.

**The first time you open it:** PQ Studio isn't signed with a paid Apple
developer certificate, so macOS says it can't check the app for malicious
software and won't open it. This is expected, and you only have to deal
with it once.

- **macOS 14 Sonoma and earlier:** in your **Applications** folder, hold
  **Control** and click **PQ Studio** (or right-click it), choose **Open**,
  then click **Open** in the message.
- **macOS 15 Sequoia and later:** double-click PQ Studio once; macOS will
  refuse, so click **Done**. Open **System Settings → Privacy & Security**,
  scroll down to **Security**, click **Open Anyway** next to the message
  about PQ Studio, and confirm with your password.
- **Or, in Terminal:**
  `xattr -dr com.apple.quarantine "/Applications/PQ Studio.app"`

After that it opens normally. To remove it, drag it from Applications to
the Bin.

### Linux (x86_64)

1. Download `PQ-Studio-<version>-linux-x86_64.tar.gz`.
2. In a terminal, unpack it and run the installer (no root needed):

   ```
   tar xzf PQ-Studio-<version>-linux-x86_64.tar.gz
   cd PQ-Studio-<version>-linux-x86_64
   ./install.sh
   ```

3. Start **PQ Studio** from your application launcher, or run `pq-studio`.

This installs, for your user only: the app, engine and databases in
`~/.local/lib/pq-studio/`, a `pq-studio` command in `~/.local/bin/`, a
launcher entry and icons in `~/.local/share/`, and this README and the
licence files in `~/.local/share/doc/pq-studio/`. To remove it, run
`./uninstall.sh` from the same folder. Your settings and work files are
kept.

## Build from source

You need Rust (https://rustup.rs). The scripts below are in the source
repository.

- **Linux, using your own `phreeqc`:** install the engine first, either from
  your distribution (on Arch Linux, the AUR package `phreeqc`) or from the
  official source release
  (https://github.com/phreeqc-dev/phreeqc3/releases):

  ```
  tar xzf phreeqc-3.9.0-17591.tar.gz && cd phreeqc-3.9.0-17591
  ./configure --prefix="$HOME/.local" && make -j && make install
  ```

  Then run `packaging/linux/install.sh` in the source folder. It builds PQ
  Studio and installs it as above, but without a bundled engine; PQ Studio
  finds `phreeqc` on your `PATH` or in `/usr`, `/usr/local` or `~/.local`,
  and lists the databases installed next to it.
  `packaging/linux/uninstall.sh` removes it.
- **Ready-made packages:** each script downloads the official PHREEQC
  source release, checks its checksum, builds the engine and the app, and
  writes the package to `dist/out/`.
  - Linux: `packaging/linux/build-linux.sh` (needs CMake and a C++
    compiler).
  - macOS: `packaging/macos/build-macos.sh` (needs the Xcode Command Line
    Tools and CMake). To sign with a Developer ID and notarize, set
    `MACOS_SIGN_IDENTITY` and `NOTARY_PROFILE` (see the top of the script).
  - Windows: `powershell -ExecutionPolicy Bypass -File
    packaging\windows\build-windows.ps1` (needs Visual Studio 2022 or its
    Build Tools with "Desktop development with C++").

If PQ Studio can't find the engine, a **Locate phreeqc…** button appears
next to Run. The `PHREEQC_BIN` and `PHREEQC_DATABASE` environment variables
also choose the engine and the default database.

## Where things are stored

| | Linux | macOS | Windows |
|---|---|---|---|
| Settings, plot styles and plot presets | `~/.config/pqstudio/` | `~/Library/Application Support/no.laerk.PQ-Studio/` | `%APPDATA%\PQ Studio\` |
| Engine scratch folder (input, output, SELECTED_OUTPUT files of the last run) | `~/.cache/pqstudio/workdir/` | `~/Library/Caches/no.laerk.PQ-Studio/workdir/` | `%LOCALAPPDATA%\PQ Studio\cache\workdir\` |

Your own documents are wherever you save them.

## Known limitations

- Most PHREEQC keywords used for everyday modelling have forms, but not all
  of them. Keywords without a form (RATES, USER_PRINT, USER_PUNCH,
  USER_GRAPH, ISOTOPES, COPY, DELETE, RUN_CELLS and others) are kept exactly
  as written when you open and save a file, and can be edited as text, but
  have no dedicated form.
- Database definitions (PHASES, the *_SPECIES and *_MASTER_SPECIES keywords,
  PITZER, SIT) are edited as text, not forms.
- Opening and saving a PHREEQC file written elsewhere keeps its meaning: a
  block opens as a form only if PQ Studio can write it back faithfully, and
  otherwise is kept as exact text. Blocks that open as forms are saved in PQ
  Studio's own layout, so comments inside those blocks are not kept (comments
  in text blocks are). Ranges such as `SOLUTION 1-40` open as text.
- Plots are drawn from Selected Output files. USER_GRAPH charts (which embed
  BASIC code in the input) are not drawn.
- The bundled engine is built without the optional GNU MP library, so
  inverse modelling's `-multiple_precision` option is not available.
- PQ Studio was developed and used on Linux. The macOS and Windows versions
  are built from the same code but have had much less testing.

## Licence

- PQ Studio's own code and icon: MIT licence (`LICENSE`).
- The PHREEQC engine and databases: USGS User Rights Notice, reproduced in
  `NOTICE.md` together with how PQ Studio relates to the original.
- Rust libraries compiled into PQ Studio: `THIRD_PARTY_LICENSES.md`.
- DejaVu fonts used for plot text: `FONT_LICENSE.txt`.
