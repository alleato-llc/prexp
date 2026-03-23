use ratatui::style::Color;

/// Color theme for the TUI.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    /// Border color for the process list.
    pub border_process: Color,
    /// Border color for the file list.
    pub border_file: Color,
    /// Column header text color.
    pub header: Color,
    /// Selected row background.
    pub highlight_bg: Color,
    /// Inaccessible / muted text.
    pub muted: Color,
    /// Status bar background.
    pub status_bg: Color,
    /// Status bar key hint color.
    pub status_key: Color,
    /// Status message / input cursor color.
    pub accent: Color,
    /// Socket fd color in detail view.
    pub fd_socket: Color,
    /// Pipe fd color in detail view.
    pub fd_pipe: Color,
    /// Config overlay border color.
    pub config_border: Color,
}

pub const THEMES: &[Theme] = &[
    // 0: Default (Cyan)
    Theme {
        name: "Default",
        border_process: Color::Cyan,
        border_file: Color::Green,
        header: Color::Yellow,
        highlight_bg: Color::DarkGray,
        muted: Color::DarkGray,
        status_bg: Color::Black,
        status_key: Color::Cyan,
        accent: Color::Yellow,
        fd_socket: Color::Blue,
        fd_pipe: Color::Magenta,
        config_border: Color::Yellow,
    },
    // 1: Nord
    Theme {
        name: "Nord",
        border_process: Color::Rgb(136, 192, 208),  // nord8
        border_file: Color::Rgb(163, 190, 140),      // nord14
        header: Color::Rgb(235, 203, 139),            // nord13
        highlight_bg: Color::Rgb(67, 76, 94),         // nord2
        muted: Color::Rgb(76, 86, 106),               // nord3
        status_bg: Color::Rgb(46, 52, 64),             // nord0
        status_key: Color::Rgb(136, 192, 208),         // nord8
        accent: Color::Rgb(235, 203, 139),             // nord13
        fd_socket: Color::Rgb(129, 161, 193),          // nord9
        fd_pipe: Color::Rgb(180, 142, 173),            // nord15
        config_border: Color::Rgb(235, 203, 139),
    },
    // 2: Dracula
    Theme {
        name: "Dracula",
        border_process: Color::Rgb(139, 233, 253),   // cyan
        border_file: Color::Rgb(80, 250, 123),        // green
        header: Color::Rgb(241, 250, 140),             // yellow
        highlight_bg: Color::Rgb(68, 71, 90),          // current line
        muted: Color::Rgb(98, 114, 164),               // comment
        status_bg: Color::Rgb(40, 42, 54),              // background
        status_key: Color::Rgb(189, 147, 249),          // purple
        accent: Color::Rgb(255, 184, 108),              // orange
        fd_socket: Color::Rgb(139, 233, 253),           // cyan
        fd_pipe: Color::Rgb(255, 121, 198),             // pink
        config_border: Color::Rgb(241, 250, 140),
    },
    // 3: Solarized Dark
    Theme {
        name: "Solarized",
        border_process: Color::Rgb(38, 139, 210),    // blue
        border_file: Color::Rgb(133, 153, 0),         // green
        header: Color::Rgb(181, 137, 0),               // yellow
        highlight_bg: Color::Rgb(7, 54, 66),            // base02
        muted: Color::Rgb(88, 110, 117),               // base01
        status_bg: Color::Rgb(0, 43, 54),               // base03
        status_key: Color::Rgb(38, 139, 210),           // blue
        accent: Color::Rgb(181, 137, 0),                // yellow
        fd_socket: Color::Rgb(42, 161, 152),            // cyan
        fd_pipe: Color::Rgb(211, 54, 130),              // magenta
        config_border: Color::Rgb(181, 137, 0),
    },
    // 4: Monokai
    Theme {
        name: "Monokai",
        border_process: Color::Rgb(102, 217, 239),   // cyan
        border_file: Color::Rgb(166, 226, 46),        // green
        header: Color::Rgb(230, 219, 116),             // yellow
        highlight_bg: Color::Rgb(62, 61, 50),
        muted: Color::Rgb(117, 113, 94),
        status_bg: Color::Rgb(39, 40, 34),
        status_key: Color::Rgb(249, 38, 114),          // pink
        accent: Color::Rgb(253, 151, 31),               // orange
        fd_socket: Color::Rgb(102, 217, 239),
        fd_pipe: Color::Rgb(174, 129, 255),             // purple
        config_border: Color::Rgb(230, 219, 116),
    },
    // 5: Gruvbox
    Theme {
        name: "Gruvbox",
        border_process: Color::Rgb(131, 165, 152),   // aqua
        border_file: Color::Rgb(184, 187, 38),        // green
        header: Color::Rgb(250, 189, 47),              // yellow
        highlight_bg: Color::Rgb(60, 56, 54),          // bg1
        muted: Color::Rgb(146, 131, 116),              // gray
        status_bg: Color::Rgb(40, 40, 40),              // bg
        status_key: Color::Rgb(131, 165, 152),          // aqua
        accent: Color::Rgb(254, 128, 25),               // orange
        fd_socket: Color::Rgb(131, 165, 152),
        fd_pipe: Color::Rgb(211, 134, 155),             // purple
        config_border: Color::Rgb(250, 189, 47),
    },
    // 6: Tokyo Night
    Theme {
        name: "Tokyo Night",
        border_process: Color::Rgb(125, 207, 255),   // blue
        border_file: Color::Rgb(158, 206, 106),       // green
        header: Color::Rgb(224, 175, 104),             // yellow
        highlight_bg: Color::Rgb(41, 46, 66),
        muted: Color::Rgb(86, 95, 137),
        status_bg: Color::Rgb(26, 27, 38),
        status_key: Color::Rgb(187, 154, 247),         // purple
        accent: Color::Rgb(255, 158, 100),              // orange
        fd_socket: Color::Rgb(125, 207, 255),
        fd_pipe: Color::Rgb(187, 154, 247),
        config_border: Color::Rgb(224, 175, 104),
    },
    // 7: Retro (neon green on black)
    Theme {
        name: "Retro",
        border_process: Color::Rgb(0, 255, 65),       // phosphor green
        border_file: Color::Rgb(0, 255, 65),
        header: Color::Rgb(0, 255, 65),
        highlight_bg: Color::Rgb(0, 50, 10),
        muted: Color::Rgb(0, 120, 30),
        status_bg: Color::Rgb(0, 0, 0),
        status_key: Color::Rgb(50, 255, 100),
        accent: Color::Rgb(0, 255, 65),
        fd_socket: Color::Rgb(0, 200, 50),
        fd_pipe: Color::Rgb(0, 180, 40),
        config_border: Color::Rgb(0, 255, 65),
    },
    // 8: Royal Purple
    Theme {
        name: "Royal Purple",
        border_process: Color::Rgb(155, 89, 182),     // amethyst
        border_file: Color::Rgb(192, 132, 252),        // lavender
        header: Color::Rgb(243, 208, 255),              // light orchid
        highlight_bg: Color::Rgb(48, 20, 72),           // deep purple
        muted: Color::Rgb(108, 72, 138),
        status_bg: Color::Rgb(25, 10, 40),              // near-black purple
        status_key: Color::Rgb(192, 132, 252),          // lavender
        accent: Color::Rgb(243, 208, 255),              // light orchid
        fd_socket: Color::Rgb(129, 178, 245),           // periwinkle
        fd_pipe: Color::Rgb(249, 150, 180),             // rose
        config_border: Color::Rgb(192, 132, 252),
    },
];
