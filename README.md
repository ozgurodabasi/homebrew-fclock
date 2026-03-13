# fclock

A full-screen terminal clock and countdown timer that scales to fill any terminal size — like a screensaver for your command line.

![fclock](screens/screen1.png)

---

## Features

- Big, bold clock that fills your entire terminal
- Countdown timer with visual alerts
- Stopwatch with lap recording
- GMT offset clock — e.g. `--gmt +2`
- Custom label above the clock — e.g. `--label "Tokyo time"`
- Matrix digital rain mode
- 32 named colours + rainbow mode
- Thin 7-segment LCD style option
- Runs a custom script when countdown completes
- Prints remaining time and session duration when you quit

---

## Installation

### Homebrew

```bash
brew tap ozgurodabasi/fclock
brew install fclock
```

### Build from source

Requires [Rust](https://rustup.rs).

```bash
git clone https://github.com/ozgurodabasi/homebrew-fclock
cd homebrew-fclock
cargo install --path .
```

---

## Usage

```bash
fclock [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--countdown H:MM:SS` | Start a countdown timer |
| `--stopwatch` | Start a stopwatch (counts up, centiseconds always shown) |
| `--gmt <offset>` | Show time at a GMT offset, e.g. `--gmt +2` or `--gmt -8` |
| `--label <text>` | Show a label above the clock, e.g. `--label "Tokyo time"` |
| `--showms` | Show centiseconds |
| `--color <name>` | Set the clock colour |
| `--rainbow` | Cycle through colours every second |
| `--matrix` | Matrix digital rain background |
| `--thinner` | 7-segment LCD style digits |
| `--runoncomplete <script>` | Run a script on countdown complete, or on quit for stopwatch/clock |
| `--version` | Print version and exit |

Press **`q`** or **`Esc`** to quit. Press **`Space`** during a countdown or stopwatch to record a lap time. On quit, laps, the last time, and total session duration are printed to the terminal.

---

## Help

```
$ fclock --help

fclock 0.1.2 — full-screen terminal clock and countdown timer

USAGE
  fclock [OPTIONS]

OPTIONS
  --countdown H:MM:SS     Start a countdown timer
  --stopwatch             Start a stopwatch (counts up, centiseconds always shown)
  --gmt <offset>          Show time at a GMT offset, e.g. --gmt +2 or --gmt -8
  --label <text>          Show a label above the clock, e.g. --label "Tokyo time"
  --showms                Show centiseconds
  --color <name>          Set the clock colour
  --rainbow               Cycle through colours every second
  --matrix                Matrix digital rain background
  --thinner               7-segment LCD style digits
  --runoncomplete <path>  Run a script on countdown complete, or on quit for stopwatch/clock
  --version               Print version and exit
  --help                  Show this help

COLOURS
  red green blue yellow cyan magenta white grey
  darkred darkgreen darkblue darkyellow darkcyan darkmagenta darkgrey
  purple violet orange pink hotpink lime mint teal
  navy maroon gold amber coral skyblue lavender

EXAMPLES
  fclock
  fclock --countdown 25:00
  fclock --countdown 5:00 --color orange
  fclock --matrix --countdown 10:00
  fclock --countdown 5:00 --runoncomplete ./notify.sh
  fclock --stopwatch
  fclock --gmt +9 --label "Tokyo"
  fclock --gmt -5 --label "New York"

Press q or Esc to quit. Last time and session duration are printed on exit.
Press Space during a countdown or stopwatch to record a lap time.
```

---

## Examples

```bash
# Show the current time
fclock

# Countdown from 25 minutes (Pomodoro)
fclock --countdown 25:00

# Countdown with a custom colour
fclock --countdown 5:00 --color orange

# Matrix rain clock
fclock --matrix

# Matrix rain countdown
fclock --matrix --countdown 10:00

# Rainbow colours
fclock --rainbow

# Thin LCD style
fclock --thinner

# Show centiseconds
fclock --showms

# Run a script when the countdown ends
fclock --countdown 5:00 --runoncomplete ./notify.sh

# Stopwatch with laps
fclock --stopwatch

# Show Tokyo time
fclock --gmt +9 --label "Tokyo"

# Show New York time
fclock --gmt -5 --label "New York"
```

---

## Countdown format

| Example | Duration |
|---------|----------|
| `90` | 90 seconds |
| `1:30` | 1 minute 30 seconds |
| `1:30:00` | 1 hour 30 minutes |
| `1:0:30:00` | 1 day 30 minutes |

---

## Countdown colours

The clock colour changes automatically as the countdown progresses:

| Time remaining | Colour |
|----------------|--------|
| Normal | Your chosen colour (or default cyan) |
| Last 10 seconds | Yellow |
| Expired | Red |

`--color` and `--rainbow` are overridden by these alerts.

---

## Available colours

`red` `green` `blue` `yellow` `cyan` `magenta` `white` `grey`
`darkred` `darkgreen` `darkblue` `darkyellow` `darkcyan` `darkmagenta` `darkgrey`
`purple` `violet` `orange` `pink` `hotpink` `lime` `mint` `teal`
`navy` `maroon` `gold` `amber` `coral` `skyblue` `lavender`

---

## Run a script on exit

Use `--runoncomplete` to trigger any executable when fclock exits. Your script receives:

| Variable | Value |
|----------|-------|
| `FCLOCK_TIME` | Last displayed time, e.g. `00:07:43` |
| `FCLOCK_EVENT` | `complete` (countdown hit zero) or `quit` (user quit) |

In countdown mode the script fires when the timer reaches zero. In stopwatch and clock modes it fires when you press `q`.

```bash
fclock --countdown 5:00 --runoncomplete ./notify.sh
```

```bash
#!/usr/bin/env bash
# notify.sh — example completion script
osascript -e 'display notification "Time is up!" with title "fclock"'
```

---

## Quit and capture

When you press `q`, fclock prints laps (if any), the last displayed time, and how long the app ran:

```bash
$ fclock --stopwatch
# press space twice, then q...
lap 1: 00:00:08.42
lap 2: 00:00:21.17
00:00:35.03
ran for 00:00:35
```

You can capture the output in a script:

```bash
remaining=$(fclock --countdown 10:00)
echo "Stopped with $remaining remaining"
```

---

## License

This project is licensed under the [GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0.html).

You are free to use, modify, and distribute this software under the terms of the GPL v3. Any derivative work must also be distributed under the same license.
