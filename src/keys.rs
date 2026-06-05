#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyCodeTableEnt {
    pub name: &'static str,
    pub alt_name: Option<&'static str>,
    pub shifted_name: Option<&'static str>,
}

pub const MOD_ALT: u8 = 0x01;
pub const MOD_SUPER: u8 = 0x02;
pub const MOD_SHIFT: u8 = 0x04;
pub const MOD_CTRL: u8 = 0x08;
pub const MOD_ALT_GR: u8 = 0x10;

pub struct Modifier {
    pub mask: u8,
    pub key: u8,
}

pub const MODIFIERS: [Modifier; 5] = [
    Modifier { mask: MOD_ALT, key: 56 }, // KEYD_LEFTALT
    Modifier { mask: MOD_ALT_GR, key: 100 }, // KEYD_RIGHTALT
    Modifier { mask: MOD_SHIFT, key: 42 }, // KEYD_LEFTSHIFT
    Modifier { mask: MOD_SUPER, key: 125 }, // KEYD_LEFTMETA
    Modifier { mask: MOD_CTRL, key: 29 }, // KEYD_LEFTCTRL
];

pub const KEYCODE_TABLE: [Option<KeyCodeTableEnt>; 256] = {
    let mut table = [None; 256];
    
    table[1] = Some(KeyCodeTableEnt { name: "esc", alt_name: Some("escape"), shifted_name: None });
    table[2] = Some(KeyCodeTableEnt { name: "1", alt_name: None, shifted_name: Some("!") });
    table[3] = Some(KeyCodeTableEnt { name: "2", alt_name: None, shifted_name: Some("@") });
    table[4] = Some(KeyCodeTableEnt { name: "3", alt_name: None, shifted_name: Some("#") });
    table[5] = Some(KeyCodeTableEnt { name: "4", alt_name: None, shifted_name: Some("$") });
    table[6] = Some(KeyCodeTableEnt { name: "5", alt_name: None, shifted_name: Some("%") });
    table[7] = Some(KeyCodeTableEnt { name: "6", alt_name: None, shifted_name: Some("^") });
    table[8] = Some(KeyCodeTableEnt { name: "7", alt_name: None, shifted_name: Some("&") });
    table[9] = Some(KeyCodeTableEnt { name: "8", alt_name: None, shifted_name: Some("*") });
    table[10] = Some(KeyCodeTableEnt { name: "9", alt_name: None, shifted_name: Some("(") });
    table[11] = Some(KeyCodeTableEnt { name: "0", alt_name: None, shifted_name: Some(")") });
    table[12] = Some(KeyCodeTableEnt { name: "-", alt_name: Some("minus"), shifted_name: Some("_") });
    table[13] = Some(KeyCodeTableEnt { name: "=", alt_name: Some("equal"), shifted_name: Some("+") });
    table[14] = Some(KeyCodeTableEnt { name: "backspace", alt_name: None, shifted_name: None });
    table[15] = Some(KeyCodeTableEnt { name: "tab", alt_name: None, shifted_name: None });
    table[16] = Some(KeyCodeTableEnt { name: "q", alt_name: None, shifted_name: Some("Q") });
    table[17] = Some(KeyCodeTableEnt { name: "w", alt_name: None, shifted_name: Some("W") });
    table[18] = Some(KeyCodeTableEnt { name: "e", alt_name: None, shifted_name: Some("E") });
    table[19] = Some(KeyCodeTableEnt { name: "r", alt_name: None, shifted_name: Some("R") });
    table[20] = Some(KeyCodeTableEnt { name: "t", alt_name: None, shifted_name: Some("T") });
    table[21] = Some(KeyCodeTableEnt { name: "y", alt_name: None, shifted_name: Some("Y") });
    table[22] = Some(KeyCodeTableEnt { name: "u", alt_name: None, shifted_name: Some("U") });
    table[23] = Some(KeyCodeTableEnt { name: "i", alt_name: None, shifted_name: Some("I") });
    table[24] = Some(KeyCodeTableEnt { name: "o", alt_name: None, shifted_name: Some("O") });
    table[25] = Some(KeyCodeTableEnt { name: "p", alt_name: None, shifted_name: Some("P") });
    table[26] = Some(KeyCodeTableEnt { name: "[", alt_name: Some("leftbrace"), shifted_name: Some("{") });
    table[27] = Some(KeyCodeTableEnt { name: "]", alt_name: Some("rightbrace"), shifted_name: Some("}") });
    table[28] = Some(KeyCodeTableEnt { name: "enter", alt_name: None, shifted_name: None });
    table[29] = Some(KeyCodeTableEnt { name: "leftcontrol", alt_name: Some(""), shifted_name: None });
    table[84] = Some(KeyCodeTableEnt { name: "iso-level3-shift", alt_name: None, shifted_name: None });
    table[30] = Some(KeyCodeTableEnt { name: "a", alt_name: None, shifted_name: Some("A") });
    table[31] = Some(KeyCodeTableEnt { name: "s", alt_name: None, shifted_name: Some("S") });
    table[32] = Some(KeyCodeTableEnt { name: "d", alt_name: None, shifted_name: Some("D") });
    table[33] = Some(KeyCodeTableEnt { name: "f", alt_name: None, shifted_name: Some("F") });
    table[34] = Some(KeyCodeTableEnt { name: "g", alt_name: None, shifted_name: Some("G") });
    table[35] = Some(KeyCodeTableEnt { name: "h", alt_name: None, shifted_name: Some("H") });
    table[36] = Some(KeyCodeTableEnt { name: "j", alt_name: None, shifted_name: Some("J") });
    table[37] = Some(KeyCodeTableEnt { name: "k", alt_name: None, shifted_name: Some("K") });
    table[38] = Some(KeyCodeTableEnt { name: "l", alt_name: None, shifted_name: Some("L") });
    table[39] = Some(KeyCodeTableEnt { name: ";", alt_name: Some("semicolon"), shifted_name: Some(":") });
    table[40] = Some(KeyCodeTableEnt { name: "'", alt_name: Some("apostrophe"), shifted_name: Some("\"") });
    table[41] = Some(KeyCodeTableEnt { name: "`", alt_name: Some("grave"), shifted_name: Some("~") });
    table[42] = Some(KeyCodeTableEnt { name: "leftshift", alt_name: Some(""), shifted_name: None });
    table[43] = Some(KeyCodeTableEnt { name: "\\", alt_name: Some("backslash"), shifted_name: Some("|") });
    table[44] = Some(KeyCodeTableEnt { name: "z", alt_name: None, shifted_name: Some("Z") });
    table[45] = Some(KeyCodeTableEnt { name: "x", alt_name: None, shifted_name: Some("X") });
    table[46] = Some(KeyCodeTableEnt { name: "c", alt_name: None, shifted_name: Some("C") });
    table[47] = Some(KeyCodeTableEnt { name: "v", alt_name: None, shifted_name: Some("V") });
    table[48] = Some(KeyCodeTableEnt { name: "b", alt_name: None, shifted_name: Some("B") });
    table[49] = Some(KeyCodeTableEnt { name: "n", alt_name: None, shifted_name: Some("N") });
    table[50] = Some(KeyCodeTableEnt { name: "m", alt_name: None, shifted_name: Some("M") });
    table[51] = Some(KeyCodeTableEnt { name: ",", alt_name: Some("comma"), shifted_name: Some("<") });
    table[52] = Some(KeyCodeTableEnt { name: ".", alt_name: Some("dot"), shifted_name: Some(">") });
    table[53] = Some(KeyCodeTableEnt { name: "/", alt_name: Some("slash"), shifted_name: Some("?") });
    table[54] = Some(KeyCodeTableEnt { name: "rightshift", alt_name: None, shifted_name: None });
    table[55] = Some(KeyCodeTableEnt { name: "kpasterisk", alt_name: None, shifted_name: None });
    table[56] = Some(KeyCodeTableEnt { name: "leftalt", alt_name: Some(""), shifted_name: None });
    table[57] = Some(KeyCodeTableEnt { name: "space", alt_name: None, shifted_name: None });
    table[58] = Some(KeyCodeTableEnt { name: "capslock", alt_name: None, shifted_name: None });
    table[59] = Some(KeyCodeTableEnt { name: "f1", alt_name: None, shifted_name: None });
    table[60] = Some(KeyCodeTableEnt { name: "f2", alt_name: None, shifted_name: None });
    table[61] = Some(KeyCodeTableEnt { name: "f3", alt_name: None, shifted_name: None });
    table[62] = Some(KeyCodeTableEnt { name: "f4", alt_name: None, shifted_name: None });
    table[63] = Some(KeyCodeTableEnt { name: "f5", alt_name: None, shifted_name: None });
    table[64] = Some(KeyCodeTableEnt { name: "f6", alt_name: None, shifted_name: None });
    table[65] = Some(KeyCodeTableEnt { name: "f7", alt_name: None, shifted_name: None });
    table[66] = Some(KeyCodeTableEnt { name: "f8", alt_name: None, shifted_name: None });
    table[67] = Some(KeyCodeTableEnt { name: "f9", alt_name: None, shifted_name: None });
    table[68] = Some(KeyCodeTableEnt { name: "f10", alt_name: None, shifted_name: None });
    table[69] = Some(KeyCodeTableEnt { name: "numlock", alt_name: None, shifted_name: None });
    table[70] = Some(KeyCodeTableEnt { name: "scrolllock", alt_name: None, shifted_name: None });
    table[71] = Some(KeyCodeTableEnt { name: "kp7", alt_name: None, shifted_name: None });
    table[72] = Some(KeyCodeTableEnt { name: "kp8", alt_name: None, shifted_name: None });
    table[73] = Some(KeyCodeTableEnt { name: "kp9", alt_name: None, shifted_name: None });
    table[74] = Some(KeyCodeTableEnt { name: "kpminus", alt_name: None, shifted_name: None });
    table[75] = Some(KeyCodeTableEnt { name: "kp4", alt_name: None, shifted_name: None });
    table[76] = Some(KeyCodeTableEnt { name: "kp5", alt_name: None, shifted_name: None });
    table[77] = Some(KeyCodeTableEnt { name: "kp6", alt_name: None, shifted_name: None });
    table[78] = Some(KeyCodeTableEnt { name: "kpplus", alt_name: None, shifted_name: None });
    table[79] = Some(KeyCodeTableEnt { name: "kp1", alt_name: None, shifted_name: None });
    table[80] = Some(KeyCodeTableEnt { name: "kp2", alt_name: None, shifted_name: None });
    table[81] = Some(KeyCodeTableEnt { name: "kp3", alt_name: None, shifted_name: None });
    table[82] = Some(KeyCodeTableEnt { name: "kp0", alt_name: None, shifted_name: None });
    table[83] = Some(KeyCodeTableEnt { name: "kpdot", alt_name: None, shifted_name: None });
    table[85] = Some(KeyCodeTableEnt { name: "zenkakuhankaku", alt_name: None, shifted_name: None });
    table[86] = Some(KeyCodeTableEnt { name: "102nd", alt_name: None, shifted_name: None });
    table[87] = Some(KeyCodeTableEnt { name: "f11", alt_name: None, shifted_name: None });
    table[88] = Some(KeyCodeTableEnt { name: "f12", alt_name: None, shifted_name: None });
    table[89] = Some(KeyCodeTableEnt { name: "ro", alt_name: None, shifted_name: None });
    table[90] = Some(KeyCodeTableEnt { name: "katakana", alt_name: None, shifted_name: None });
    table[91] = Some(KeyCodeTableEnt { name: "hiragana", alt_name: None, shifted_name: None });
    table[92] = Some(KeyCodeTableEnt { name: "henkan", alt_name: None, shifted_name: None });
    table[93] = Some(KeyCodeTableEnt { name: "katakanahiragana", alt_name: None, shifted_name: None });
    table[94] = Some(KeyCodeTableEnt { name: "muhenkan", alt_name: None, shifted_name: None });
    table[95] = Some(KeyCodeTableEnt { name: "kpjpcomma", alt_name: None, shifted_name: None });
    table[96] = Some(KeyCodeTableEnt { name: "kpenter", alt_name: None, shifted_name: None });
    table[97] = Some(KeyCodeTableEnt { name: "rightcontrol", alt_name: None, shifted_name: None });
    table[98] = Some(KeyCodeTableEnt { name: "kpslash", alt_name: None, shifted_name: None });
    table[99] = Some(KeyCodeTableEnt { name: "sysrq", alt_name: None, shifted_name: None });
    table[100] = Some(KeyCodeTableEnt { name: "rightalt", alt_name: None, shifted_name: None });
    table[101] = Some(KeyCodeTableEnt { name: "linefeed", alt_name: None, shifted_name: None });
    table[102] = Some(KeyCodeTableEnt { name: "home", alt_name: None, shifted_name: None });
    table[103] = Some(KeyCodeTableEnt { name: "up", alt_name: None, shifted_name: None });
    table[104] = Some(KeyCodeTableEnt { name: "pageup", alt_name: None, shifted_name: None });
    table[105] = Some(KeyCodeTableEnt { name: "left", alt_name: None, shifted_name: None });
    table[106] = Some(KeyCodeTableEnt { name: "right", alt_name: None, shifted_name: None });
    table[107] = Some(KeyCodeTableEnt { name: "end", alt_name: None, shifted_name: None });
    table[108] = Some(KeyCodeTableEnt { name: "down", alt_name: None, shifted_name: None });
    table[109] = Some(KeyCodeTableEnt { name: "pagedown", alt_name: None, shifted_name: None });
    table[110] = Some(KeyCodeTableEnt { name: "insert", alt_name: None, shifted_name: None });
    table[111] = Some(KeyCodeTableEnt { name: "delete", alt_name: None, shifted_name: None });
    table[112] = Some(KeyCodeTableEnt { name: "macro", alt_name: None, shifted_name: None });
    table[113] = Some(KeyCodeTableEnt { name: "mute", alt_name: None, shifted_name: None });
    table[114] = Some(KeyCodeTableEnt { name: "volumedown", alt_name: None, shifted_name: None });
    table[115] = Some(KeyCodeTableEnt { name: "volumeup", alt_name: None, shifted_name: None });
    table[116] = Some(KeyCodeTableEnt { name: "power", alt_name: None, shifted_name: None });
    table[117] = Some(KeyCodeTableEnt { name: "kpequal", alt_name: None, shifted_name: None });
    table[118] = Some(KeyCodeTableEnt { name: "kpplusminus", alt_name: None, shifted_name: None });
    table[119] = Some(KeyCodeTableEnt { name: "pause", alt_name: None, shifted_name: None });
    table[120] = Some(KeyCodeTableEnt { name: "scale", alt_name: None, shifted_name: None });
    table[121] = Some(KeyCodeTableEnt { name: "kpcomma", alt_name: None, shifted_name: None });
    table[122] = Some(KeyCodeTableEnt { name: "hangeul", alt_name: None, shifted_name: None });
    table[123] = Some(KeyCodeTableEnt { name: "hanja", alt_name: None, shifted_name: None });
    table[124] = Some(KeyCodeTableEnt { name: "yen", alt_name: None, shifted_name: None });
    table[125] = Some(KeyCodeTableEnt { name: "leftmeta", alt_name: Some(""), shifted_name: None });
    table[126] = Some(KeyCodeTableEnt { name: "rightmeta", alt_name: None, shifted_name: None });
    table[127] = Some(KeyCodeTableEnt { name: "compose", alt_name: None, shifted_name: None });
    table[128] = Some(KeyCodeTableEnt { name: "stop", alt_name: None, shifted_name: None });
    table[129] = Some(KeyCodeTableEnt { name: "again", alt_name: None, shifted_name: None });
    table[130] = Some(KeyCodeTableEnt { name: "props", alt_name: None, shifted_name: None });
    table[131] = Some(KeyCodeTableEnt { name: "undo", alt_name: None, shifted_name: None });
    table[132] = Some(KeyCodeTableEnt { name: "front", alt_name: None, shifted_name: None });
    table[133] = Some(KeyCodeTableEnt { name: "copy", alt_name: None, shifted_name: None });
    table[134] = Some(KeyCodeTableEnt { name: "open", alt_name: None, shifted_name: None });
    table[135] = Some(KeyCodeTableEnt { name: "paste", alt_name: None, shifted_name: None });
    table[136] = Some(KeyCodeTableEnt { name: "find", alt_name: None, shifted_name: None });
    table[137] = Some(KeyCodeTableEnt { name: "cut", alt_name: None, shifted_name: None });
    table[138] = Some(KeyCodeTableEnt { name: "help", alt_name: None, shifted_name: None });
    table[139] = Some(KeyCodeTableEnt { name: "menu", alt_name: None, shifted_name: None });
    table[140] = Some(KeyCodeTableEnt { name: "calc", alt_name: None, shifted_name: None });
    table[141] = Some(KeyCodeTableEnt { name: "setup", alt_name: None, shifted_name: None });
    table[142] = Some(KeyCodeTableEnt { name: "sleep", alt_name: None, shifted_name: None });
    table[143] = Some(KeyCodeTableEnt { name: "wakeup", alt_name: None, shifted_name: None });
    table[144] = Some(KeyCodeTableEnt { name: "file", alt_name: None, shifted_name: None });
    table[145] = Some(KeyCodeTableEnt { name: "sendfile", alt_name: None, shifted_name: None });
    table[146] = Some(KeyCodeTableEnt { name: "deletefile", alt_name: None, shifted_name: None });
    table[147] = Some(KeyCodeTableEnt { name: "xfer", alt_name: None, shifted_name: None });
    table[148] = Some(KeyCodeTableEnt { name: "scrolldown", alt_name: None, shifted_name: None });
    table[149] = Some(KeyCodeTableEnt { name: "scrollup", alt_name: None, shifted_name: None });
    table[150] = Some(KeyCodeTableEnt { name: "www", alt_name: None, shifted_name: None });
    table[151] = Some(KeyCodeTableEnt { name: "msdos", alt_name: None, shifted_name: None });
    table[152] = Some(KeyCodeTableEnt { name: "coffee", alt_name: None, shifted_name: None });
    table[153] = Some(KeyCodeTableEnt { name: "display", alt_name: None, shifted_name: None });
    table[154] = Some(KeyCodeTableEnt { name: "cyclewindows", alt_name: None, shifted_name: None });
    table[155] = Some(KeyCodeTableEnt { name: "mail", alt_name: None, shifted_name: None });
    table[156] = Some(KeyCodeTableEnt { name: "favorites", alt_name: Some("bookmarks"), shifted_name: None });
    table[157] = Some(KeyCodeTableEnt { name: "computer", alt_name: None, shifted_name: None });
    table[158] = Some(KeyCodeTableEnt { name: "back", alt_name: None, shifted_name: None });
    table[159] = Some(KeyCodeTableEnt { name: "forward", alt_name: None, shifted_name: None });
    table[160] = Some(KeyCodeTableEnt { name: "closecd", alt_name: None, shifted_name: None });
    table[161] = Some(KeyCodeTableEnt { name: "ejectcd", alt_name: None, shifted_name: None });
    table[162] = Some(KeyCodeTableEnt { name: "ejectclosecd", alt_name: None, shifted_name: None });
    table[163] = Some(KeyCodeTableEnt { name: "nextsong", alt_name: None, shifted_name: None });
    table[164] = Some(KeyCodeTableEnt { name: "playpause", alt_name: None, shifted_name: None });
    table[165] = Some(KeyCodeTableEnt { name: "previoussong", alt_name: None, shifted_name: None });
    table[166] = Some(KeyCodeTableEnt { name: "stopcd", alt_name: None, shifted_name: None });
    table[167] = Some(KeyCodeTableEnt { name: "record", alt_name: None, shifted_name: None });
    table[168] = Some(KeyCodeTableEnt { name: "rewind", alt_name: None, shifted_name: None });
    table[169] = Some(KeyCodeTableEnt { name: "phone", alt_name: None, shifted_name: None });
    table[170] = Some(KeyCodeTableEnt { name: "iso", alt_name: None, shifted_name: None });
    table[171] = Some(KeyCodeTableEnt { name: "config", alt_name: None, shifted_name: None });
    table[172] = Some(KeyCodeTableEnt { name: "homepage", alt_name: None, shifted_name: None });
    table[173] = Some(KeyCodeTableEnt { name: "refresh", alt_name: None, shifted_name: None });
    table[174] = Some(KeyCodeTableEnt { name: "exit", alt_name: None, shifted_name: None });
    table[175] = Some(KeyCodeTableEnt { name: "move", alt_name: None, shifted_name: None });
    table[176] = Some(KeyCodeTableEnt { name: "edit", alt_name: None, shifted_name: None });
    table[179] = Some(KeyCodeTableEnt { name: "kpleftparen", alt_name: None, shifted_name: None });
    table[180] = Some(KeyCodeTableEnt { name: "kprightparen", alt_name: None, shifted_name: None });
    table[181] = Some(KeyCodeTableEnt { name: "new", alt_name: None, shifted_name: None });
    table[182] = Some(KeyCodeTableEnt { name: "redo", alt_name: None, shifted_name: None });
    table[183] = Some(KeyCodeTableEnt { name: "f13", alt_name: None, shifted_name: None });
    table[184] = Some(KeyCodeTableEnt { name: "f14", alt_name: None, shifted_name: None });
    table[185] = Some(KeyCodeTableEnt { name: "f15", alt_name: None, shifted_name: None });
    table[186] = Some(KeyCodeTableEnt { name: "f16", alt_name: None, shifted_name: None });
    table[187] = Some(KeyCodeTableEnt { name: "f17", alt_name: None, shifted_name: None });
    table[188] = Some(KeyCodeTableEnt { name: "f18", alt_name: None, shifted_name: None });
    table[189] = Some(KeyCodeTableEnt { name: "f19", alt_name: None, shifted_name: None });
    table[190] = Some(KeyCodeTableEnt { name: "f20", alt_name: None, shifted_name: None });
    table[191] = Some(KeyCodeTableEnt { name: "f21", alt_name: Some("prog1"), shifted_name: None });
    table[192] = Some(KeyCodeTableEnt { name: "f22", alt_name: Some("prog2"), shifted_name: None });
    table[193] = Some(KeyCodeTableEnt { name: "f23", alt_name: Some("prog3"), shifted_name: None });
    table[194] = Some(KeyCodeTableEnt { name: "f24", alt_name: Some("prog4"), shifted_name: None });
    table[200] = Some(KeyCodeTableEnt { name: "playcd", alt_name: None, shifted_name: None });
    table[201] = Some(KeyCodeTableEnt { name: "pausecd", alt_name: None, shifted_name: None });
    table[202] = Some(KeyCodeTableEnt { name: "scrollright", alt_name: None, shifted_name: None });
    table[203] = Some(KeyCodeTableEnt { name: "scrollleft", alt_name: None, shifted_name: None });
    table[204] = Some(KeyCodeTableEnt { name: "dashboard", alt_name: None, shifted_name: None });
    table[205] = Some(KeyCodeTableEnt { name: "suspend", alt_name: None, shifted_name: None });
    table[206] = Some(KeyCodeTableEnt { name: "close", alt_name: None, shifted_name: None });
    table[207] = Some(KeyCodeTableEnt { name: "play", alt_name: None, shifted_name: None });
    table[208] = Some(KeyCodeTableEnt { name: "fastforward", alt_name: None, shifted_name: None });
    table[209] = Some(KeyCodeTableEnt { name: "bassboost", alt_name: None, shifted_name: None });
    table[210] = Some(KeyCodeTableEnt { name: "print", alt_name: None, shifted_name: None });
    table[211] = Some(KeyCodeTableEnt { name: "hp", alt_name: None, shifted_name: None });
    table[212] = Some(KeyCodeTableEnt { name: "camera", alt_name: None, shifted_name: None });
    table[213] = Some(KeyCodeTableEnt { name: "sound", alt_name: None, shifted_name: None });
    table[214] = Some(KeyCodeTableEnt { name: "question", alt_name: None, shifted_name: None });
    table[215] = Some(KeyCodeTableEnt { name: "email", alt_name: None, shifted_name: None });
    table[216] = Some(KeyCodeTableEnt { name: "chat", alt_name: None, shifted_name: None });
    table[217] = Some(KeyCodeTableEnt { name: "search", alt_name: None, shifted_name: None });
    table[218] = Some(KeyCodeTableEnt { name: "connect", alt_name: None, shifted_name: None });
    table[219] = Some(KeyCodeTableEnt { name: "finance", alt_name: None, shifted_name: None });
    table[220] = Some(KeyCodeTableEnt { name: "sport", alt_name: None, shifted_name: None });
    table[221] = Some(KeyCodeTableEnt { name: "shop", alt_name: None, shifted_name: None });
    table[222] = Some(KeyCodeTableEnt { name: "voicecommand", alt_name: None, shifted_name: None });
    table[223] = Some(KeyCodeTableEnt { name: "cancel", alt_name: None, shifted_name: None });
    table[224] = Some(KeyCodeTableEnt { name: "brightnessdown", alt_name: None, shifted_name: None });
    table[225] = Some(KeyCodeTableEnt { name: "brightnessup", alt_name: None, shifted_name: None });
    table[226] = Some(KeyCodeTableEnt { name: "media", alt_name: None, shifted_name: None });
    table[227] = Some(KeyCodeTableEnt { name: "switchvideomode", alt_name: None, shifted_name: None });
    table[228] = Some(KeyCodeTableEnt { name: "kbdillumtoggle", alt_name: None, shifted_name: None });
    table[229] = Some(KeyCodeTableEnt { name: "kbdillumdown", alt_name: None, shifted_name: None });
    table[230] = Some(KeyCodeTableEnt { name: "kbdillumup", alt_name: None, shifted_name: None });
    table[231] = Some(KeyCodeTableEnt { name: "send", alt_name: None, shifted_name: None });
    table[232] = Some(KeyCodeTableEnt { name: "reply", alt_name: None, shifted_name: None });
    table[233] = Some(KeyCodeTableEnt { name: "forwardmail", alt_name: None, shifted_name: None });
    table[234] = Some(KeyCodeTableEnt { name: "save", alt_name: None, shifted_name: None });
    table[235] = Some(KeyCodeTableEnt { name: "documents", alt_name: None, shifted_name: None });
    table[236] = Some(KeyCodeTableEnt { name: "battery", alt_name: None, shifted_name: None });
    table[237] = Some(KeyCodeTableEnt { name: "bluetooth", alt_name: None, shifted_name: None });
    table[238] = Some(KeyCodeTableEnt { name: "wlan", alt_name: None, shifted_name: None });
    table[239] = Some(KeyCodeTableEnt { name: "uwb", alt_name: None, shifted_name: None });
    table[240] = Some(KeyCodeTableEnt { name: "unknown", alt_name: None, shifted_name: None });
    table[241] = Some(KeyCodeTableEnt { name: "next", alt_name: None, shifted_name: None });
    table[242] = Some(KeyCodeTableEnt { name: "prev", alt_name: None, shifted_name: None });
    table[243] = Some(KeyCodeTableEnt { name: "cycle", alt_name: None, shifted_name: None });
    table[244] = Some(KeyCodeTableEnt { name: "auto", alt_name: None, shifted_name: None });
    table[245] = Some(KeyCodeTableEnt { name: "off", alt_name: None, shifted_name: None });
    table[246] = Some(KeyCodeTableEnt { name: "wwan", alt_name: None, shifted_name: None });
    table[247] = Some(KeyCodeTableEnt { name: "rfkill", alt_name: None, shifted_name: None });
    table[248] = Some(KeyCodeTableEnt { name: "micmute", alt_name: None, shifted_name: None });
    table[249] = Some(KeyCodeTableEnt { name: "leftmouse", alt_name: None, shifted_name: None });
    table[250] = Some(KeyCodeTableEnt { name: "middlemouse", alt_name: None, shifted_name: None });
    table[251] = Some(KeyCodeTableEnt { name: "rightmouse", alt_name: None, shifted_name: None });
    table[252] = Some(KeyCodeTableEnt { name: "mouse1", alt_name: None, shifted_name: None });
    table[253] = Some(KeyCodeTableEnt { name: "mouse2", alt_name: None, shifted_name: None });
    table[178] = Some(KeyCodeTableEnt { name: "mouseback", alt_name: None, shifted_name: None });
    table[255] = Some(KeyCodeTableEnt { name: "mouseforward", alt_name: None, shifted_name: None });
    table[254] = Some(KeyCodeTableEnt { name: "fn", alt_name: None, shifted_name: None });
    table[177] = Some(KeyCodeTableEnt { name: "zoom", alt_name: None, shifted_name: None });
    table[195] = Some(KeyCodeTableEnt { name: "noop", alt_name: None, shifted_name: None });

    table
};

pub fn parse_modset(s: &str) -> Option<u8> {
    let mut mods = 0;
    let mut parts = s.split('-');
    
    for part in parts {
        if part.is_empty() {
            continue;
        }
        match part {
            "C" => mods |= MOD_CTRL,
            "M" => mods |= MOD_SUPER,
            "A" => mods |= MOD_ALT,
            "S" => mods |= MOD_SHIFT,
            "G" => mods |= MOD_ALT_GR,
            _ => return None,
        }
    }
    
    Some(mods)
}

pub fn parse_key_sequence(s: &str) -> Option<(u8, u8)> {
    let mut mods = 0;
    let mut parts: Vec<&str> = s.split('-').collect();
    
    if parts.is_empty() {
        return None;
    }
    
    let key_part = parts.pop().unwrap();
    
    for part in parts {
        match part {
            "C" => mods |= MOD_CTRL,
            "M" => mods |= MOD_SUPER,
            "A" => mods |= MOD_ALT,
            "S" => mods |= MOD_SHIFT,
            "G" => mods |= MOD_ALT_GR,
            _ => return None,
        }
    }
    
    for (i, ent) in KEYCODE_TABLE.iter().enumerate() {
        if let Some(ent) = ent {
            if let Some(shifted) = ent.shifted_name {
                if shifted == key_part {
                    return Some((i as u8, mods | MOD_SHIFT));
                }
            }
            if ent.name == key_part || ent.alt_name == Some(key_part) {
                return Some((i as u8, mods));
            }
        }
    }
    
    None
}
