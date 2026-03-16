use std::env;
use std::io::{stdout, Write};
use std::time::Duration;

// ── screensaver / display-sleep prevention ───────────────────────────────────

#[cfg(target_os = "macos")]
mod screensaver {
    use std::process::{Child, Command};

    /// Spawns `caffeinate -d -i` to prevent display sleep and screensaver.
    /// Killed automatically on `Drop`.
    pub struct Guard(Child);

    impl Guard {
        pub fn new() -> Option<Self> {
            Command::new("caffeinate")
                .args(["-d", "-i"])
                .spawn()
                .ok()
                .map(Guard)
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod screensaver {
    pub struct Guard;
    impl Guard {
        pub fn new() -> Option<Self> { Some(Guard) }
    }
}

use chrono::{Local, Utc, Timelike};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

// ── block pixel font (default) ───────────────────────────────────────────────

const DIGITS: [[&str; 5]; 10] = [
    ["███", "█ █", "█ █", "█ █", "███"], // 0
    [" █ ", " █ ", " █ ", " █ ", " █ "], // 1
    ["███", "  █", "███", "█  ", "███"], // 2
    ["███", "  █", "███", "  █", "███"], // 3
    ["█ █", "█ █", "███", "  █", "  █"], // 4
    ["███", "█  ", "███", "  █", "███"], // 5
    ["███", "█  ", "███", "█ █", "███"], // 6
    ["███", "  █", "  █", "  █", "  █"], // 7
    ["███", "█ █", "███", "█ █", "███"], // 8
    ["███", "█ █", "███", "  █", "███"], // 9
];
const COLON: [&str; 5] = [" ", "█", " ", "█", " "];
const DOT:   [&str; 5] = [" ", " ", " ", " ", "█"];

fn render_px_row(row: &str, scale_x: usize) -> String {
    row.chars()
        .flat_map(|c| std::iter::repeat(if c == '█' { '█' } else { ' ' }).take(scale_x))
        .collect()
}

// ── 7-segment LCD font (--thinner) ───────────────────────────────────────────
//
// Segments: [top, upper-right, lower-right, bottom, lower-left, upper-left, middle]
//             a       b           c           d         e           f         g
//
//   ╭─a─╮
//   f   b
//   ├─g─┤
//   e   c
//   ╰─d─╯

const SEG7: [[bool; 7]; 10] = [
//    a      b      c      d      e      f      g
    [true,  true,  true,  true,  true,  true,  false], // 0
    [false, true,  true,  false, false, false, false], // 1
    [true,  true,  false, true,  true,  false, true ], // 2
    [true,  true,  true,  true,  false, false, true ], // 3
    [false, true,  true,  false, false, true,  true ], // 4
    [true,  false, true,  true,  false, true,  true ], // 5
    [true,  false, true,  true,  true,  true,  true ], // 6
    [true,  true,  true,  false, false, false, false], // 7
    [true,  true,  true,  true,  true,  true,  true ], // 8
    [true,  true,  true,  true,  false, true,  true ], // 9
];

/// Render one digit as `scale_y*2 + 3` rows, each `inner_w + 2` chars wide.
fn render_digit(d: u32, iw: usize, sy: usize) -> Vec<String> {
    let [a, b, c, bot, e, f, g] = SEG7[d as usize];
    let mut rows = Vec::with_capacity(sy * 2 + 3);

    // top bar
    rows.push(if a { format!("╭{}╮", "─".repeat(iw)) } else { " ".repeat(iw + 2) });

    // upper half
    let (ul, ur) = (if f { "│" } else { " " }, if b { "│" } else { " " });
    for _ in 0..sy { rows.push(format!("{}{}{}", ul, " ".repeat(iw), ur)); }

    // middle bar
    rows.push(if g {
        format!("{}{}{}",
            if f || e { "├" } else { "─" },
            "─".repeat(iw),
            if b || c { "┤" } else { "─" })
    } else {
        format!("{}{}{}",
            if f || e { "│" } else { " " },
            " ".repeat(iw),
            if b || c { "│" } else { " " })
    });

    // lower half
    let (ll, lr) = (if e { "│" } else { " " }, if c { "│" } else { " " });
    for _ in 0..sy { rows.push(format!("{}{}{}", ll, " ".repeat(iw), lr)); }

    // bottom bar
    rows.push(if bot { format!("╰{}╯", "─".repeat(iw)) } else { " ".repeat(iw + 2) });
    rows
}

/// Colon: 3 chars wide, same height as a digit. Two ● dots centred in each half.
fn render_colon(sy: usize) -> Vec<String> {
    let h = sy * 2 + 3;
    let mut rows: Vec<String> = (0..h).map(|_| "   ".into()).collect();
    rows[1 + sy / 2]         = " ● ".into();
    rows[sy + 2 + sy / 2]    = " ● ".into();
    rows
}

/// Decimal dot for --showms: 3 chars wide, single ● near the bottom.
fn render_dot(sy: usize) -> Vec<String> {
    let h = sy * 2 + 3;
    let mut rows: Vec<String> = (0..h).map(|_| "   ".into()).collect();
    rows[h - 2] = " ● ".into();
    rows
}

// ── named colours ────────────────────────────────────────────────────────────

const NAMED_COLORS: &[(&str, Color)] = &[
    ("red",         Color::Red),
    ("green",       Color::Green),
    ("blue",        Color::Blue),
    ("yellow",      Color::Yellow),
    ("cyan",        Color::Cyan),
    ("magenta",     Color::Magenta),
    ("white",       Color::White),
    ("grey",        Color::Grey),
    ("gray",        Color::Grey),
    ("darkred",     Color::DarkRed),
    ("darkgreen",   Color::DarkGreen),
    ("darkblue",    Color::DarkBlue),
    ("darkyellow",  Color::DarkYellow),
    ("darkcyan",    Color::DarkCyan),
    ("darkmagenta", Color::DarkMagenta),
    ("darkgrey",    Color::DarkGrey),
    ("darkgray",    Color::DarkGrey),
    ("purple",      Color::AnsiValue(129)),
    ("violet",      Color::AnsiValue(177)),
    ("orange",      Color::AnsiValue(208)),
    ("pink",        Color::AnsiValue(205)),
    ("hotpink",     Color::AnsiValue(198)),
    ("lime",        Color::AnsiValue(118)),
    ("mint",        Color::AnsiValue(121)),
    ("teal",        Color::AnsiValue(37)),
    ("navy",        Color::DarkBlue),
    ("maroon",      Color::DarkRed),
    ("gold",        Color::AnsiValue(220)),
    ("amber",       Color::AnsiValue(214)),
    ("coral",       Color::AnsiValue(209)),
    ("skyblue",     Color::AnsiValue(117)),
    ("lavender",    Color::AnsiValue(183)),
];

const RAINBOW: &[Color] = &[
    Color::Red,
    Color::AnsiValue(208),
    Color::Yellow,
    Color::AnsiValue(118),
    Color::Green,
    Color::Cyan,
    Color::AnsiValue(117),
    Color::Blue,
    Color::AnsiValue(129),
    Color::Magenta,
    Color::AnsiValue(205),
    Color::AnsiValue(214),
];

fn color_from_name(name: &str) -> Option<Color> {
    let lower = name.to_lowercase();
    NAMED_COLORS.iter().find(|(n, _)| *n == lower.as_str()).map(|(_, c)| *c)
}

fn rainbow_color(second: u64) -> Color {
    let h = second.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    RAINBOW[(h % RAINBOW.len() as u64) as usize]
}

// ── matrix rain ──────────────────────────────────────────────────────────────

// Half-width katakana + ASCII symbols — all 1-cell wide
const MATRIX_CHARS: &[char] = &[
    '2','3','4','5','7','8','Z','T','H','X','C','E','K','M','R',
    ':','+','=','*','|','.','"','<','>',
    'ｦ','ｧ','ｨ','ｩ','ｪ','ｫ','ｬ','ｭ','ｮ','ｯ',
    'ｱ','ｲ','ｳ','ｴ','ｵ','ｶ','ｷ','ｸ','ｹ','ｺ',
    'ｻ','ｼ','ｽ','ｾ','ｿ','ﾀ','ﾁ','ﾂ','ﾃ','ﾄ',
    'ﾅ','ﾆ','ﾇ','ﾈ','ﾉ','ﾊ','ﾋ','ﾌ','ﾍ','ﾎ',
];

struct Rng(u64);
impl Rng {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(42)
            .wrapping_add(0x517cc1b727220a95);
        Self(seed)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
    fn between(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % (hi - lo) as u64) as i32
    }
}

struct Drop {
    head:      i32,
    trail_len: usize,
    chars:     Vec<char>,
    speed_inv: u32,   // advance head every N ticks
    tick:      u32,
}

impl Drop {
    fn new(rng: &mut Rng, h: usize, scattered: bool) -> Self {
        let trail_len = rng.between(6, 24) as usize;
        let chars = (0..h.max(64))
            .map(|_| MATRIX_CHARS[rng.below(MATRIX_CHARS.len())])
            .collect();
        let speed_inv = rng.between(1, 4) as u32;
        let head = if scattered {
            rng.between(0, h as i32)
        } else {
            -(trail_len as i32) - rng.between(0, 20)
        };
        Drop { head, trail_len, chars, speed_inv, tick: 0 }
    }

    fn update(&mut self, rng: &mut Rng) {
        self.tick += 1;
        if self.tick < self.speed_inv { return; }
        self.tick = 0;
        self.head += 1;
        // randomly flicker a character inside the visible trail
        if rng.below(3) == 0 && !self.chars.is_empty() {
            let lo = (self.head - self.trail_len as i32).max(0) as usize;
            let hi = self.head.max(0) as usize;
            if lo < hi && hi < self.chars.len() {
                let idx = lo + rng.below(hi - lo);
                self.chars[idx] = MATRIX_CHARS[rng.below(MATRIX_CHARS.len())];
            }
        }
    }

    fn is_done(&self, term_h: u16) -> bool {
        self.head > term_h as i32 + self.trail_len as i32
    }

    /// Returns (char, depth_from_head) if this drop covers `row`.
    fn char_at(&self, row: i32) -> Option<(char, i32)> {
        let depth = self.head - row;
        if depth >= 0 && (depth as usize) < self.trail_len {
            let ch = self.chars.get(row as usize).copied().unwrap_or('|');
            Some((ch, depth))
        } else {
            None
        }
    }
}

struct Matrix {
    drops: Vec<Option<Drop>>,
    rng:   Rng,
    w:     u16,
    h:     u16,
}

impl Matrix {
    fn new(w: u16, h: u16) -> Self {
        let mut rng = Rng::new();
        let drops = (0..w as usize)
            .map(|_| {
                if rng.below(3) == 0 {
                    Some(Drop::new(&mut rng, h as usize, true))
                } else {
                    None
                }
            })
            .collect();
        Matrix { drops, rng, w, h }
    }

    fn resize(&mut self, w: u16, h: u16) {
        if w != self.w || h != self.h {
            *self = Matrix::new(w, h);
        }
    }

    fn update(&mut self) {
        let h = self.h;
        for i in 0..self.drops.len() {
            let done = self.drops[i].as_mut().map_or(false, |d| {
                d.update(&mut self.rng);
                d.is_done(h)
            });
            if done { self.drops[i] = None; }
            if self.drops[i].is_none() && self.rng.below(50) == 0 {
                self.drops[i] = Some(Drop::new(&mut self.rng, h as usize, false));
            }
        }
    }

    fn render(&self, stdout: &mut impl Write) -> std::io::Result<()> {
        for (col, drop_opt) in self.drops.iter().enumerate() {
            if let Some(drop) = drop_opt {
                for row in 0..self.h as i32 {
                    if let Some((ch, depth)) = drop.char_at(row) {
                        let color = match depth {
                            0     => Color::White,
                            1..=2 => Color::Green,
                            3..=7 => Color::DarkGreen,
                            _     => Color::AnsiValue(22),
                        };
                        queue!(stdout,
                            MoveTo(col as u16, row as u16),
                            SetForegroundColor(color),
                            Print(ch),
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
}

// ── clock segments ───────────────────────────────────────────────────────────

enum Seg { Digit(u32), Colon, Dot }

impl Seg {
    // ── block mode ──
    fn px_width(&self) -> usize { match self { Seg::Digit(_) => 3, _ => 1 } }
    fn block_rows(&self) -> &'static [&'static str; 5] {
        match self {
            Seg::Digit(d) => &DIGITS[*d as usize],
            Seg::Colon    => &COLON,
            Seg::Dot      => &DOT,
        }
    }

    // ── thinner (7-seg) mode ──
    fn thin_render(&self, inner_w: usize, scale_y: usize) -> Vec<String> {
        match self {
            Seg::Digit(d) => render_digit(*d, inner_w, scale_y),
            Seg::Colon    => render_colon(scale_y),
            Seg::Dot      => render_dot(scale_y),
        }
    }
    fn thin_width(&self, inner_w: usize) -> usize {
        match self { Seg::Digit(_) => inner_w + 2, _ => 3 }
    }
}

fn build_segs(h: u32, m: u32, s: u32, ms: Option<u32>) -> Vec<Seg> {
    let mut segs = vec![
        Seg::Digit(h / 10), Seg::Digit(h % 10),
        Seg::Colon,
        Seg::Digit(m / 10), Seg::Digit(m % 10),
        Seg::Colon,
        Seg::Digit(s / 10), Seg::Digit(s % 10),
    ];
    if let Some(millis) = ms {
        let cs = millis / 10;
        segs.push(Seg::Dot);
        segs.push(Seg::Digit(cs / 10));
        segs.push(Seg::Digit(cs % 10));
    }
    segs
}

/// Returns `(scale_param, scale_y)`.
/// Block mode:   scale_param = scale_x  (pixels per logical pixel, width)
/// Thinner mode: scale_param = inner_w  (inner width of each digit in chars)
fn compute_scale(term_w: u16, term_h: u16, with_ms: bool, thinner: bool) -> (usize, usize) {
    if thinner {
        let scale_y = ((term_h as usize * 3 / 4).saturating_sub(3) / 2).max(1);
        let ideal   = scale_y * 2 + 1;
        // HH:MM:SS → 6*(iw+2) + 2*3 + 9 = 6iw+27 ; with ms → 8iw+38
        let (fixed, dcnt) = if with_ms { (38usize, 8usize) } else { (27usize, 6usize) };
        let max_iw  = (term_w as usize).saturating_sub(fixed) / dcnt;
        (ideal.min(max_iw).max(2), scale_y)
    } else {
        let logical_w = if with_ms { 37usize } else { 27usize };
        let scale_x   = ((term_w as usize) / logical_w).max(1);
        let scale_y   = ((term_h as usize * 3 / 4) / 5).max(1);
        (scale_x, scale_y)
    }
}

fn logical_width(segs: &[Seg], param: usize, thinner: bool) -> usize {
    if thinner {
        segs.iter().map(|s| s.thin_width(param)).sum::<usize>() + segs.len().saturating_sub(1)
    } else {
        let px: usize = segs.iter().map(|s| s.px_width()).sum();
        (px + segs.len().saturating_sub(1)) * param
    }
}

fn render_clock(
    stdout:    &mut impl Write,
    segs:      &[Seg],
    col:       usize,
    start_row: usize,
    param:     usize,   // scale_x (block) or inner_w (thinner)
    scale_y:   usize,
    color:     Color,
    bold:      bool,
    thinner:   bool,
) -> std::io::Result<()> {
    if bold { queue!(stdout, SetAttribute(Attribute::Bold))?; }

    if thinner {
        let rendered: Vec<Vec<String>> = segs.iter().map(|s| s.thin_render(param, scale_y)).collect();
        for row in 0..scale_y * 2 + 3 {
            let mut c = col;
            for (si, (seg, rows)) in segs.iter().zip(rendered.iter()).enumerate() {
                if si > 0 { c += 1; }
                queue!(stdout,
                    MoveTo(c as u16, (start_row + row) as u16),
                    SetForegroundColor(color),
                    Print(&rows[row]),
                )?;
                c += seg.thin_width(param);
            }
        }
    } else {
        let mut term_row = start_row;
        for px_row in 0..5usize {
            for _ in 0..scale_y {
                let mut c = col;
                for (si, seg) in segs.iter().enumerate() {
                    if si > 0 { c += param; }
                    let rendered = render_px_row(seg.block_rows()[px_row], param);
                    queue!(stdout,
                        MoveTo(c as u16, term_row as u16),
                        SetForegroundColor(color),
                        Print(&rendered),
                    )?;
                    c += seg.px_width() * param;
                }
                term_row += 1;
            }
        }
    }

    if bold { queue!(stdout, SetAttribute(Attribute::Reset))?; }
    Ok(())
}

// ── argument parsing helpers ─────────────────────────────────────────────────

fn parse_countdown(s: &str) -> u64 {
    let parts: Vec<u64> = s.split(':').map(|x| x.parse().unwrap_or(0)).collect();
    match parts.len() {
        1 => parts[0],
        2 => parts[0] * 60 + parts[1],
        3 => parts[0] * 3600 + parts[1] * 60 + parts[2],
        4 => parts[0] * 86400 + parts[1] * 3600 + parts[2] * 60 + parts[3],
        _ => 0,
    }
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut show_ms          = false;
    let mut rainbow          = false;
    let mut matrix           = false;
    let mut thinner          = false;
    let mut stopwatch        = false;
    let mut gmt_offset: Option<i32> = None;
    let mut label: Option<String>   = None;
    let mut custom_color: Option<Color>  = None;
    let mut countdown_secs: Option<u64>  = None;
    let mut on_complete: Option<String>  = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--version" | "-V" | "-v" => {
                println!("fclock {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "help" | "-h" => {
                println!("fclock {} — full-screen terminal clock and countdown timer\n", env!("CARGO_PKG_VERSION"));
                println!("USAGE");
                println!("  fclock [OPTIONS]\n");
                println!("OPTIONS");
                println!("  --countdown H:MM:SS     Start a countdown timer");
                println!("  --stopwatch             Start a stopwatch (counts up, centiseconds always shown)");
                println!("  --gmt <offset>          Show time at a GMT offset, e.g. --gmt +2 or --gmt -8");
                println!("  --label <text>          Show a label above the clock, e.g. --label \"Tokyo time\"");
                println!("  --showms                Show centiseconds");
                println!("  --color <name>          Set the clock colour");
                println!("  --rainbow               Cycle through colours every second");
                println!("  --matrix                Matrix digital rain background");
                println!("  --thinner               7-segment LCD style digits");
                println!("  --runoncomplete <path>  Run a script on countdown complete or on quit (stopwatch/clock)");
                println!("  --version               Print version and exit");
                println!("  --help                  Show this help\n");
                println!("COLOURS");
                println!("  red green blue yellow cyan magenta white grey");
                println!("  darkred darkgreen darkblue darkyellow darkcyan darkmagenta darkgrey");
                println!("  purple violet orange pink hotpink lime mint teal");
                println!("  navy maroon gold amber coral skyblue lavender\n");
                println!("EXAMPLES");
                println!("  fclock");
                println!("  fclock --countdown 25:00");
                println!("  fclock --countdown 5:00 --color orange");
                println!("  fclock --matrix --countdown 10:00");
                println!("  fclock --countdown 5:00 --runoncomplete ./notify.sh");
                println!("  fclock --stopwatch");
                println!("  fclock --gmt +9 --label \"Tokyo\"");
                println!("  fclock --gmt -5 --label \"New York\"\n");
                println!("Press q or Esc to quit. Last time and session duration are printed on exit.");
                println!("Press Space during a countdown or stopwatch to record a lap time.");
                return Ok(());
            }
            "--showms"      => show_ms   = true,
            "--rainbow"     => rainbow   = true,
            "--matrix"      => matrix    = true,
            "--thinner"     => thinner   = true,
            "--stopwatch"   => stopwatch = true,
            "--gmt" => {
                if i + 1 < args.len() {
                    i += 1;
                    gmt_offset = args[i].parse::<i32>().ok();
                }
            }
            "--label" => {
                if i + 1 < args.len() {
                    i += 1;
                    label = Some(args[i].clone());
                }
            }
            "--color"       => {
                if i + 1 < args.len() {
                    i += 1;
                    match color_from_name(&args[i]) {
                        Some(c) => custom_color = Some(c),
                        None    => eprintln!("unknown color '{}', using default", args[i]),
                    }
                }
            }
            "--countdown"   => {
                if i + 1 < args.len() {
                    i += 1;
                    countdown_secs = Some(parse_countdown(&args[i]));
                }
            }
            "--runoncomplete" => {
                if i + 1 < args.len() {
                    i += 1;
                    on_complete = Some(args[i].clone());
                }
            }
            _ => {}
        }
        i += 1;
    }

    let start = std::time::Instant::now();
    let _no_sleep = screensaver::Guard::new(); // prevent display sleep / screensaver
    let mut stdout = std::io::BufWriter::new(stdout());
    execute!(stdout, EnterAlternateScreen, Hide)?;
    terminal::enable_raw_mode()?;

    let (result, last_time, laps) =
        run_loop(&mut stdout, show_ms, rainbow, matrix, thinner, stopwatch, gmt_offset, label, custom_color, countdown_secs, on_complete, start);

    terminal::disable_raw_mode()?;
    execute!(stdout, Show, LeaveAlternateScreen)?;

    for (i, lap) in laps.iter().enumerate() {
        println!("lap {}: {}", i + 1, lap);
    }

    if let Some(t) = last_time {
        println!("{}", t);
    }

    let elapsed = start.elapsed();
    let es = elapsed.as_secs();
    println!("ran for {:02}:{:02}:{:02}", es / 3600, es / 60 % 60, es % 60);

    result
}

// ── render loop ──────────────────────────────────────────────────────────────

fn run_loop(
    stdout:         &mut impl Write,
    show_ms:        bool,
    rainbow:        bool,
    matrix:         bool,
    thinner:        bool,
    stopwatch:      bool,
    gmt_offset:     Option<i32>,
    label:          Option<String>,
    custom_color:   Option<Color>,
    countdown_secs: Option<u64>,
    on_complete:    Option<String>,
    start:          std::time::Instant,
) -> (std::io::Result<()>, Option<String>, Vec<String>) {
    let (mut tw, mut th) = match terminal::size() {
        Ok(s) => s,
        Err(e) => return (Err(e), None, Vec::new()),
    };
    let mut mat = matrix.then(|| Matrix::new(tw, th));
    let mut last_time: Option<String> = None;
    let mut script_fired = false;
    let mut laps: Vec<String> = Vec::new();

    // Caches to avoid re-allocating on frames where nothing changed.
    let mut prev_render_key = (u32::MAX, u32::MAX, u32::MAX, u32::MAX, false);
    let mut laps_cached_len = 0usize;
    let mut lap_str_cache = String::new();

    // Pre-compute the hint string — it never changes during a run.
    let hint: String = if countdown_secs.is_some() || stopwatch {
        "space: lap  q: quit".to_owned()
    } else if let Some(off) = gmt_offset {
        format!("GMT{:+}  q: quit", off)
    } else {
        "q: quit".to_owned()
    };

    loop {
        // faster tick in matrix mode for smooth rain
        let timeout = if matrix { 33 } else { 50 };
        match poll(Duration::from_millis(timeout)) {
            Err(e) => return (Err(e), last_time, laps),
            Ok(true) => match read() {
                Err(e) => return (Err(e), last_time, laps),
                Ok(Event::Key(KeyEvent { code: KeyCode::Char('q'), .. }))
                | Ok(Event::Key(KeyEvent { code: KeyCode::Char('Q'), .. }))
                | Ok(Event::Key(KeyEvent { code: KeyCode::Esc,      .. })) => break,
                Ok(Event::Key(KeyEvent { code: KeyCode::Char(' '), .. })) => {
                    if countdown_secs.is_some() || stopwatch {
                        if let Some(ref t) = last_time {
                            laps.push(t.clone());
                        }
                    }
                }
                Ok(Event::Resize(w, h)) => {
                    tw = w; th = h;
                    if let Some(m) = &mut mat { m.resize(w, h); }
                }
                _ => {}
            },
            Ok(false) => {}
        }

        // ── time ────────────────────────────────────────────────────────────
        let (h, m, s, ms, done) = if stopwatch {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let elapsed_s  = elapsed_ms / 1000;
            ((elapsed_s / 3600) as u32, (elapsed_s / 60 % 60) as u32,
             (elapsed_s % 60) as u32, (elapsed_ms % 1000) as u32, false)
        } else {
            match countdown_secs {
                Some(total) => {
                    let elapsed_ms = start.elapsed().as_millis() as u64;
                    let total_ms   = total * 1000;
                    if elapsed_ms >= total_ms {
                        (0u32, 0u32, 0u32, 0u32, true)
                    } else {
                        let rem_ms = total_ms - elapsed_ms;
                        let rem_s  = rem_ms / 1000;
                        ((rem_s / 3600) as u32, (rem_s / 60 % 60) as u32,
                         (rem_s % 60) as u32, (rem_ms % 1000) as u32, false)
                    }
                }
                None => {
                    if let Some(offset_h) = gmt_offset {
                        let now = Utc::now() + chrono::Duration::hours(offset_h as i64);
                        (now.hour(), now.minute(), now.second(), now.nanosecond() / 1_000_000, false)
                    } else {
                        let now = Local::now();
                        (now.hour(), now.minute(), now.second(), now.nanosecond() / 1_000_000, false)
                    }
                }
            }
        };

        // Build the human-readable time string — only when the displayed value changes.
        let with_ms = show_ms || stopwatch;
        let cs = ms / 10; // centiseconds (resolution shown on screen)
        let render_key = (h, m, s, if with_ms { cs } else { 0 }, done);
        let time_changed = render_key != prev_render_key;
        let laps_changed = laps.len() != laps_cached_len;

        if time_changed {
            prev_render_key = render_key;
            last_time = Some(if with_ms {
                format!("{:02}:{:02}:{:02}.{:02}", h, m, s, cs)
            } else {
                format!("{:02}:{:02}:{:02}", h, m, s)
            });
        }

        // Fire script once when countdown reaches zero.
        if done && !script_fired {
            script_fired = true;
            if let Some(ref path) = on_complete {
                let _ = std::process::Command::new(path)
                    .env("FCLOCK_TIME", last_time.as_deref().unwrap_or(""))
                    .env("FCLOCK_EVENT", "complete")
                    .spawn();
            }
        }

        // Rebuild lap string only when a new lap is recorded.
        if laps_changed {
            laps_cached_len = laps.len();
            lap_str_cache = laps.iter().enumerate()
                .map(|(i, t)| format!("lap {}: {}", i + 1, t))
                .collect::<Vec<_>>()
                .join("  —  ");
        }

        // Skip redraw entirely when nothing visible has changed (no matrix animation).
        if !matrix && !time_changed && !laps_changed {
            continue;
        }

        let ms_opt = if with_ms { Some(ms) } else { None };
        let segs   = build_segs(h, m, s, ms_opt);

        let (param, scale_y) = compute_scale(tw, th, with_ms, thinner);
        let dw  = logical_width(&segs, param, thinner);
        let dh  = if thinner { scale_y * 2 + 3 } else { 5 * scale_y };
        let col = (tw as usize).saturating_sub(dw) / 2;
        let row = (th as usize).saturating_sub(dh) / 2;

        // ── colour ──────────────────────────────────────────────────────────
        let wall_sec = Local::now().timestamp() as u64;
        let time_left = countdown_secs.unwrap_or(0).saturating_sub(start.elapsed().as_secs());

        let clock_color = if matrix {
            // matrix mode: bright green clock floating over the rain;
            // flash red when countdown expires
            if done { Color::Red } else { Color::AnsiValue(46) }
        } else if done {
            Color::Red
        } else if countdown_secs.is_some() && time_left <= 10 {
            Color::Yellow
        } else if rainbow {
            rainbow_color(wall_sec)
        } else if let Some(c) = custom_color {
            c
        } else {
            match (countdown_secs, stopwatch) {
                (None, false) => Color::Green,
                (_, true)     => Color::White,
                _             => Color::Cyan,
            }
        };

        // ── draw ────────────────────────────────────────────────────────────
        macro_rules! try_io {
            ($e:expr) => { match $e { Ok(v) => v, Err(e) => return (Err(e), last_time, laps) } };
        }

        try_io!(queue!(stdout, Clear(ClearType::All)));

        if let Some(mat) = &mut mat {
            mat.update();
            try_io!(mat.render(stdout));
        }

        if let Some(ref lbl) = label {
            let lx = (tw as usize).saturating_sub(lbl.len()) / 2;
            let ly = row.saturating_sub(2);
            try_io!(queue!(stdout,
                MoveTo(lx as u16, ly as u16),
                SetForegroundColor(Color::DarkGrey),
                Print(lbl),
                ResetColor,
            ));
        }

        if !lap_str_cache.is_empty() {
            let lx = (tw as usize).saturating_sub(lap_str_cache.len()) / 2;
            let clock_h = if thinner { scale_y * 2 + 3 } else { 5 * scale_y };
            let ly = row + clock_h + 1;
            if ly < th as usize {
                try_io!(queue!(stdout,
                    MoveTo(lx as u16, ly as u16),
                    SetForegroundColor(Color::DarkGrey),
                    Print(&lap_str_cache),
                    ResetColor,
                ));
            }
        }

        try_io!(render_clock(stdout, &segs, col, row, param, scale_y, clock_color, matrix, thinner));

        try_io!(queue!(
            stdout,
            ResetColor,
            MoveTo((tw as usize).saturating_sub(hint.len()) as u16, th - 1),
            SetForegroundColor(Color::DarkGrey),
            Print(hint.as_str()),
            ResetColor,
        ));

        if let Err(e) = stdout.flush() { return (Err(e), last_time, laps); }
    }

    // Fire script on quit for non-countdown modes.
    if countdown_secs.is_none() {
        if let Some(ref path) = on_complete {
            let _ = std::process::Command::new(path)
                .env("FCLOCK_TIME", last_time.as_deref().unwrap_or(""))
                .env("FCLOCK_EVENT", "quit")
                .spawn();
        }
    }

    (Ok(()), last_time, laps)
}
