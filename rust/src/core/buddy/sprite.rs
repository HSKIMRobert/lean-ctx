use super::types::{BuddyState, CreatureTraits, Mood};

pub(super) struct SpritePack {
    pub(super) base: Vec<String>,
    pub(super) frames: Vec<Vec<String>>,
    pub(super) anim_ms: Option<u32>,
}

pub(super) fn sprite_tier(level: u32) -> u8 {
    if level >= 75 {
        4
    } else if level >= 50 {
        3
    } else if level >= 25 {
        2
    } else {
        u8::from(level >= 10)
    }
}

fn tier_anim_ms(tier: u8) -> Option<u32> {
    match tier {
        0 => None,
        1 => Some(950),
        2 => Some(700),
        3 => Some(520),
        _ => Some(380),
    }
}

pub(super) fn render_sprite_pack(traits: &CreatureTraits, mood: &Mood, level: u32) -> SpritePack {
    let base = render_sprite(traits, mood);
    let tier = sprite_tier(level);
    if tier == 0 {
        return SpritePack {
            base,
            frames: Vec::new(),
            anim_ms: None,
        };
    }

    let mut frames = Vec::new();
    frames.push(base.clone());

    let blink = match mood {
        Mood::Sleeping => ("u", "u"),
        _ => (".", "."),
    };
    frames.push(render_sprite_with_eyes(traits, mood, blink.0, blink.1));

    if tier >= 2 {
        let mut s = base.clone();
        if let Some(l0) = s.get_mut(0) {
            *l0 = sparkle_edges(l0, '*', '+');
        }
        frames.push(s);
    }
    if tier >= 3 {
        let mut s = base.clone();
        for line in &mut s {
            *line = shift(line, 1);
        }
        frames.push(s);
    }
    if tier >= 4 {
        let mut s = base.clone();
        for (i, line) in s.iter_mut().enumerate() {
            let (l, r) = if i % 2 == 0 { ('+', '+') } else { ('*', '*') };
            *line = edge_aura(line, l, r);
        }
        frames.push(s);
    }

    SpritePack {
        base,
        frames,
        anim_ms: tier_anim_ms(tier),
    }
}

fn render_sprite_with_eyes(
    traits: &CreatureTraits,
    _mood: &Mood,
    el: &str,
    er: &str,
) -> Vec<String> {
    let ears = ear_part(traits.ears);
    let head_top = head_top_part(traits.head);
    let face = face_line(traits.head, traits.eyes, el, er);
    let mouth = mouth_line(traits.head, traits.mouth);
    let neck = neck_part(traits.head);
    let body = body_part(traits.body, traits.markings);
    let feet = leg_part(traits.legs, traits.tail);

    vec![
        pad(&ears),
        pad(&head_top),
        pad(&face),
        pad(&mouth),
        pad(&neck),
        pad(&body),
        pad(&feet),
    ]
}

pub(super) fn sparkle_edges(line: &str, left: char, right: char) -> String {
    let s = pad(line);
    let mut chars: Vec<char> = s.chars().collect();
    if chars.len() >= 2 {
        chars[0] = left;
        let last = chars.len() - 1;
        chars[last] = right;
    }
    chars.into_iter().collect()
}

pub(super) fn edge_aura(line: &str, left: char, right: char) -> String {
    let s = pad(line);
    let mut chars: Vec<char> = s.chars().collect();
    if chars.len() >= 2 {
        chars[0] = left;
        let last = chars.len() - 1;
        chars[last] = right;
    }
    chars.into_iter().collect()
}

pub(super) fn shift(line: &str, offset: i32) -> String {
    if offset == 0 {
        return pad(line);
    }
    let s = pad(line);
    let mut chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return s;
    }
    if offset > 0 {
        for _ in 0..offset {
            chars.insert(0, ' ');
            chars.pop();
        }
    } else {
        for _ in 0..(-offset) {
            chars.remove(0);
            chars.push(' ');
        }
    }
    chars.into_iter().collect()
}

pub(super) fn sprite_lines_for_tick(state: &BuddyState, tick: Option<u64>) -> &[String] {
    if let Some(t) = tick {
        if !state.ascii_frames.is_empty() {
            let idx = (t as usize) % state.ascii_frames.len();
            return &state.ascii_frames[idx];
        }
    }
    &state.ascii_art
}

const W: usize = 20;

fn pad(s: &str) -> String {
    let len = s.chars().count();
    if len >= W {
        s.chars().take(W).collect()
    } else {
        let left = (W - len) / 2;
        let right = W - len - left;
        format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
    }
}

pub fn render_sprite(traits: &CreatureTraits, mood: &Mood) -> Vec<String> {
    let (el, er) = mood_eyes(mood);
    let ears = ear_part(traits.ears);
    let head_top = head_top_part(traits.head);
    let face = face_line(traits.head, traits.eyes, el, er);
    let mouth = mouth_line(traits.head, traits.mouth);
    let neck = neck_part(traits.head);
    let body = body_part(traits.body, traits.markings);
    let feet = leg_part(traits.legs, traits.tail);

    vec![
        pad(&ears),
        pad(&head_top),
        pad(&face),
        pad(&mouth),
        pad(&neck),
        pad(&body),
        pad(&feet),
    ]
}

pub(super) fn mood_eyes(mood: &Mood) -> (&'static str, &'static str) {
    match mood {
        Mood::Ecstatic => ("*", "*"),
        Mood::Happy => ("o", "o"),
        Mood::Content => ("-", "-"),
        Mood::Worried => (">", "<"),
        Mood::Sleeping => ("u", "u"),
    }
}

fn ear_part(idx: u8) -> String {
    match idx % 12 {
        0 => r"  /\    /\".into(),
        1 => r" /  \  /  \".into(),
        2 => r"  ()    ()".into(),
        3 => r"  ||    ||".into(),
        4 => r" ~'      '~".into(),
        5 => r"  >>    <<".into(),
        6 => r"  **    **".into(),
        7 => r" .'      '.".into(),
        8 => r"  ~~    ~~".into(),
        9 => r"  ^^    ^^".into(),
        10 => r"  {}    {}".into(),
        _ => r"  <>    <>".into(),
    }
}

fn head_top_part(idx: u8) -> String {
    match idx % 12 {
        0 => " .--------. ".into(),
        1 => " +--------+ ".into(),
        2 => " /--------\\ ".into(),
        3 => " .========. ".into(),
        4 => " (--------) ".into(),
        5 => " .~~~~~~~~. ".into(),
        6 => " /~~~~~~~~\\ ".into(),
        7 => " {--------} ".into(),
        8 => " <--------> ".into(),
        9 => " .'^----^'. ".into(),
        10 => " /********\\ ".into(),
        _ => " (________) ".into(),
    }
}

fn head_bracket(head: u8) -> (char, char) {
    match head % 12 {
        0 | 1 | 3 | 5 => ('|', '|'),
        2 | 6 | 10 => ('/', '\\'),
        7 => ('{', '}'),
        8 => ('<', '>'),
        _ => ('(', ')'),
    }
}

fn face_line(head: u8, eye_idx: u8, el: &str, er: &str) -> String {
    let (bl, br) = head_bracket(head);
    let deco = match eye_idx % 10 {
        1 => ("'", "'"),
        2 => (".", "."),
        3 => ("~", "~"),
        4 => ("*", "*"),
        5 => ("`", "`"),
        6 => ("^", "^"),
        7 => (",", ","),
        8 => (":", ":"),
        _ => (" ", " "),
    };
    format!(" {bl}  {}{el}  {er}{}  {br} ", deco.0, deco.1)
}

fn mouth_line(head: u8, mouth: u8) -> String {
    let (bl, br) = head_bracket(head);
    let m = match mouth % 10 {
        0 => " \\_/  ",
        1 => "  w   ",
        2 => "  ^   ",
        3 => "  ~   ",
        4 => " ===  ",
        5 => "  o   ",
        6 => "  3   ",
        7 => "  v   ",
        8 => " ---  ",
        _ => "  U   ",
    };
    format!(" {bl}  {m}  {br} ")
}

fn neck_part(head: u8) -> String {
    match head % 12 {
        0 => " '--------' ".into(),
        1 => " +--------+ ".into(),
        2 => " \\--------/ ".into(),
        3 => " '========' ".into(),
        4 => " (--------) ".into(),
        5 => " '~~~~~~~~' ".into(),
        6 => " \\~~~~~~~~/ ".into(),
        7 => " {--------} ".into(),
        8 => " <--------> ".into(),
        9 => " '.^----^.' ".into(),
        10 => " \\********/ ".into(),
        _ => " (__________) ".into(),
    }
}

fn body_part(body: u8, markings: u8) -> String {
    let fill = match markings % 6 {
        0 => "      ",
        1 => " |||| ",
        2 => " .... ",
        3 => " >><< ",
        4 => " ~~~~ ",
        _ => " :::: ",
    };
    match body % 10 {
        0 | 8 => format!("  /{fill}\\  "),
        1 | 7 => format!("  |{fill}|  "),
        2 => format!("  ({fill})  "),
        3 => format!("  [{fill}]  "),
        4 => format!("  ~{fill}~  "),
        5 => format!("  <{fill}>  "),
        6 => format!("  {{{fill}}}  "),
        _ => format!("  _{fill}_  "),
    }
}

pub(super) fn leg_part(legs: u8, tail: u8) -> String {
    let t = match tail % 8 {
        0 => ' ',
        1 => '~',
        2 => '>',
        3 => ')',
        4 => '^',
        5 => '*',
        6 => '=',
        _ => '/',
    };
    let base = match legs % 10 {
        0 => " /|      |\\",
        1 => " ~~      ~~",
        2 => "_/|      |\\_",
        3 => " ||      ||",
        4 => " /\\      /\\",
        5 => " <>      <>",
        6 => " ()      ()",
        7 => " }{      }{",
        8 => " //      \\\\",
        _ => " \\/      \\/",
    };
    if t == ' ' {
        pad(base)
    } else {
        pad(&format!("{base} {t}"))
    }
}
