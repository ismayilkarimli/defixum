//! The messages shown on the pause screen, and the check for "type to unlock" prompts.

use crate::rules::PromptKind;
use serde::Serialize;

/// What the overlay shows: a headline, plus (for `Commit`) a sentence to type out.
#[derive(Serialize, Clone, Debug)]
pub struct Prompt {
    pub kind: PromptKind,
    /// The headline, or for `Commit` the instruction above the input.
    pub text: String,
    /// An exact sentence the user has to copy out.
    pub sentence: Option<String>,
}

impl Prompt {
    /// What to remember as "the last prompt" so the next pause can avoid it. The sentence
    /// wins when there is one, since two sentence prompts share the same instruction.
    pub fn memo(&self) -> String {
        self.sentence.clone().unwrap_or_else(|| self.text.clone())
    }
}

/// What the conditions and placeholders can draw on.
pub struct Ctx<'a> {
    pub site: &'a str,
    /// Local hour, 0–23.
    pub hour: u32,
    pub opens_today: u32,
    pub minutes_today: u64,
    /// Stands in for "how long a visit usually lasts": the longest "open for" choice.
    pub longest_session: u32,
}

struct Msg {
    kind: PromptKind,
    text: &'static str,
    sentence: Option<&'static str>,
    when: fn(&Ctx) -> bool,
}

const fn msg(kind: PromptKind, text: &'static str, when: fn(&Ctx) -> bool) -> Msg {
    Msg { kind, text, sentence: None, when }
}

/// "Type to unlock": an admission the user has to write out in full before the site opens.
const fn commit(sentence: &'static str, when: fn(&Ctx) -> bool) -> Msg {
    Msg { kind: PromptKind::Commit, text: SENTENCE_TEXT, sentence: Some(sentence), when }
}

fn always(_: &Ctx) -> bool {
    true
}
fn night(c: &Ctx) -> bool {
    c.hour >= 22 || c.hour < 5
}
fn morning(c: &Ctx) -> bool {
    (5..10).contains(&c.hour)
}
fn repeat_visit(c: &Ctx) -> bool {
    c.opens_today >= 2
}
fn long_day(c: &Ctx) -> bool {
    c.minutes_today >= 10
}

const SENTENCE_TEXT: &str = "To unlock, type this sentence exactly:";

static CATALOG: &[Msg] = &[
    // Reflect
    msg(PromptKind::Reflect, "Breathe in. Breathe out.", always),
    msg(PromptKind::Reflect, "Take a deep breath. Is this what you want to do right now?", always),
    msg(PromptKind::Reflect, "What were you about to do before you opened {site}?", always),
    msg(PromptKind::Reflect, "Notice the urge. You don't have to act on it.", always),
    msg(PromptKind::Reflect, "How do you want to feel when you close this?", always),
    msg(PromptKind::Reflect, "What do you actually need right now?", always),
    msg(PromptKind::Reflect, "Is this a choice, or a habit?", always),
    // Count the cost
    msg(PromptKind::Cost, "Is {minutes} minutes of scrolling worth giving up {minutes} minutes of sleep tonight?", night),
    msg(PromptKind::Cost, "It's late. Will this help you rest, or keep you up?", night),
    msg(PromptKind::Cost, "The first hour sets the tone for your day. Is this how you want to start it?", morning),
    msg(PromptKind::Cost, "The algorithm is capitalizing on your attention. What are you getting out of this exchange?", always),
    msg(PromptKind::Cost, "{site} is built to keep you here as long as possible. You decide when to leave.", always),
    msg(PromptKind::Cost, "Every minute here is one you won't get back. Is it worth it?", always),
    msg(PromptKind::Cost, "This is visit number {visit} to {site} today. What did the last one give you?", repeat_visit),
    msg(PromptKind::Cost, "{today} minutes on {site} today already. What else could that time have been?", long_day),
    // Type to unlock
    commit("I am choosing to spend my finite time on {site}.", always),
    commit("I could close this tab, but I'm choosing {site} instead.", always),
    commit("I know this won't make me happier, and I'm opening it anyway.", always),
    commit("I'm trading time I will never get back for {site}.", always),
    commit("I had better things to do, and I picked {site}.", always),
    commit("Future me will wish I had closed this tab.", always),
    commit("I'm letting {site} decide how I spend my day.", always),
    commit("I'm giving up sleep for {site}.", night),
    commit("I've already been here {opens} times today, and I'm back for more.", repeat_visit),
    // Type to unlock, the encouraging kind: a reminder of what the time could go to instead
    commit("I am destined to do great things.", always),
    commit("I am capable of more than I give myself credit for.", always),
    commit("My time is valuable, and I decide where it goes.", always),
    commit("I am building a life I am proud of.", always),
    commit("I have the discipline to do what matters to me.", always),
    commit("Small choices like this one shape who I become.", always),
    commit("I am in control of my attention.", always),
    commit("I deserve a mind that is rested and clear.", always),
    commit("Tomorrow I will thank myself for resting tonight.", night),
    commit("Today is a good day to make progress on my goals.", morning),
];

/// Small splitmix64-seeded xorshift; plenty for choosing a message.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Rng((z ^ (z >> 31)) | 1)
    }

    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

fn fill(template: &str, c: &Ctx) -> String {
    template
        .replace("{site}", c.site)
        .replace("{minutes}", &c.longest_session.to_string())
        .replace("{visit}", &(c.opens_today + 1).to_string())
        .replace("{opens}", &c.opens_today.to_string())
        .replace("{today}", &c.minutes_today.to_string())
}

/// Picks a category at random from `kinds`, then a message from it that fits `ctx`, avoiding
/// `last` (see [`Prompt::memo`]) when something else is available.
pub fn pick(kinds: &[PromptKind], ctx: &Ctx, last: Option<&str>, seed: u64) -> Prompt {
    pick_from(CATALOG, kinds, ctx, last, seed)
}

fn pick_from(catalog: &[Msg], kinds: &[PromptKind], ctx: &Ctx, last: Option<&str>, seed: u64) -> Prompt {
    let mut rng = Rng::new(seed);
    let mut unique: Vec<PromptKind> = vec![];
    for &k in kinds {
        if !unique.contains(&k) {
            unique.push(k);
        }
    }
    let kind = if unique.is_empty() { PromptKind::Reflect } else { unique[rng.below(unique.len())] };

    let eligible = |kind: PromptKind| -> Vec<&Msg> {
        catalog.iter().filter(|m| m.kind == kind && (m.when)(ctx)).collect()
    };
    let mut options = eligible(kind);
    if options.is_empty() {
        options = eligible(PromptKind::Reflect);
    }
    let build = |m: &Msg| Prompt {
        kind: m.kind,
        text: fill(m.text, ctx),
        sentence: m.sentence.map(|s| fill(s, ctx)),
    };
    let mut prompts: Vec<Prompt> = options.into_iter().map(build).collect();
    if let Some(last) = last {
        if prompts.iter().any(|p| p.memo() != last) {
            prompts.retain(|p| p.memo() != last);
        }
    }
    if prompts.is_empty() {
        // not reachable with the real catalog, which always has Reflect messages
        return Prompt { kind: PromptKind::Reflect, text: "Breathe in. Breathe out.".into(), sentence: None };
    }
    let n = rng.below(prompts.len());
    prompts.swap_remove(n)
}

fn normalize(s: &str) -> String {
    let s: String = s
        .trim()
        .chars()
        .map(|c| match c {
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201C}' | '\u{201D}' => '"',
            c => c,
        })
        .collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    s.strip_suffix('.').unwrap_or(&s).trim_end().to_string()
}

/// Has the user typed enough to continue? The overlay mirrors these rules to enable its buttons.
pub fn unlocks(p: &Prompt, typed: &str) -> bool {
    if let Some(sentence) = &p.sentence {
        return normalize(typed) == normalize(sentence);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(hour: u32) -> Ctx<'static> {
        Ctx { site: "youtube.com", hour, opens_today: 0, minutes_today: 0, longest_session: 15 }
    }

    fn all_kinds() -> [PromptKind; 3] {
        [PromptKind::Reflect, PromptKind::Cost, PromptKind::Commit]
    }

    fn is_night_text(t: &str) -> bool {
        t.contains("of sleep tonight") || t.contains("It's late")
    }

    #[test]
    fn night_messages_only_at_night() {
        for seed in 0..500 {
            let p = pick(&[PromptKind::Cost], &ctx(15), None, seed);
            assert!(!is_night_text(&p.text), "{}", p.text);
            assert!(!p.text.contains("first hour"), "{}", p.text);
        }
        let seen: Vec<String> = (0..500).map(|s| pick(&[PromptKind::Cost], &ctx(23), None, s).text).collect();
        assert!(seen.iter().any(|t| is_night_text(t)));
        assert!(seen.iter().all(|t| !t.contains("first hour")));
        // early morning hours count as night, 5 o'clock does not
        assert!((0..500).any(|s| is_night_text(&pick(&[PromptKind::Cost], &ctx(3), None, s).text)));
        assert!((0..500).all(|s| !is_night_text(&pick(&[PromptKind::Cost], &ctx(5), None, s).text)));
        assert!((0..500).any(|s| pick(&[PromptKind::Cost], &ctx(5), None, s).text.contains("first hour")));
        assert!((0..500).all(|s| !pick(&[PromptKind::Cost], &ctx(10), None, s).text.contains("first hour")));
    }

    #[test]
    fn placeholders_are_filled() {
        let c = Ctx { site: "reddit.com", hour: 23, opens_today: 3, minutes_today: 42, longest_session: 20 };
        let mut saw = (false, false, false, false);
        for seed in 0..2000 {
            let p = pick(&all_kinds(), &c, None, seed);
            for t in [Some(&p.text), p.sentence.as_ref()].into_iter().flatten() {
                assert!(!t.contains('{') && !t.contains('}'), "{t}");
                saw.0 |= t.contains("reddit.com");
                saw.1 |= t.contains("20 minutes");
                saw.2 |= t.contains("visit number 4");
                saw.3 |= t.contains("42 minutes on reddit.com");
            }
        }
        assert_eq!(saw, (true, true, true, true));
    }

    #[test]
    fn conditions_on_opens_and_minutes() {
        let visit = |p: &Prompt| p.text.contains("visit number");
        let today = |p: &Prompt| p.text.contains("minutes on youtube.com today");
        let mut c = ctx(15);
        c.opens_today = 1;
        c.minutes_today = 9;
        for seed in 0..500 {
            let p = pick(&[PromptKind::Cost], &c, None, seed);
            assert!(!visit(&p) && !today(&p), "{}", p.text);
        }
        c.opens_today = 2;
        c.minutes_today = 10;
        let picks: Vec<Prompt> = (0..500).map(|s| pick(&[PromptKind::Cost], &c, None, s)).collect();
        assert!(picks.iter().any(visit));
        assert!(picks.iter().any(today));
        assert!(picks.iter().any(|p| p.text.contains("visit number 3")));
    }

    #[test]
    fn empty_kinds_means_reflect() {
        let reflect = ["Breathe in", "deep breath", "about to do", "urge", "How do you want", "actually need", "habit"];
        for seed in 0..100 {
            let p = pick(&[], &ctx(12), None, seed);
            assert_eq!(p.kind, PromptKind::Reflect);
            assert!(reflect.iter().any(|r| p.text.contains(r)), "{}", p.text);
        }
    }

    #[test]
    fn categories_are_balanced_and_last_is_avoided() {
        let mut counts = [0; 3];
        for seed in 0..3000 {
            match pick(&all_kinds(), &ctx(12), None, seed).kind {
                PromptKind::Reflect => counts[0] += 1,
                PromptKind::Cost => counts[1] += 1,
                PromptKind::Commit => counts[2] += 1,
            }
        }
        assert!(counts.iter().all(|&n| n > 800), "{counts:?}");

        let last = "Is this a choice, or a habit?";
        for seed in 0..300 {
            assert_ne!(pick(&[PromptKind::Reflect], &ctx(12), Some(last), seed).text, last);
        }
        let last = "I am choosing to spend my finite time on youtube.com.";
        for seed in 0..300 {
            let p = pick(&[PromptKind::Commit], &ctx(12), Some(last), seed);
            assert_ne!(p.sentence.as_deref(), Some(last));
        }
    }

    #[test]
    fn falls_back_to_reflect() {
        let catalog = [
            msg(PromptKind::Reflect, "Breathe.", always),
            msg(PromptKind::Cost, "Never.", |_| false),
        ];
        for seed in 0..50 {
            let p = pick_from(&catalog, &[PromptKind::Cost], &ctx(12), None, seed);
            assert_eq!((p.kind, p.text.as_str()), (PromptKind::Reflect, "Breathe."));
        }
    }

    #[test]
    fn sentence_unlock_is_forgiving() {
        let p = Prompt {
            kind: PromptKind::Commit,
            text: SENTENCE_TEXT.into(),
            sentence: Some("I could close this tab, but I'm choosing youtube.com instead.".into()),
        };
        assert!(unlocks(&p, "I could close this tab, but I'm choosing youtube.com instead."));
        assert!(unlocks(&p, "i could close this tab, but i’m choosing youtube.com instead"));
        assert!(unlocks(&p, "  I could   close this tab, but I‘m choosing YouTube.com   instead.  "));
        assert!(!unlocks(&p, ""));
        assert!(!unlocks(&p, "I could close this tab, but I'm choosing youtube.com"));
        assert!(!unlocks(&p, "I could close this tab, but I'm choosing reddit.com instead."));
        // quotes
        let q = Prompt { sentence: Some("It's “fine”.".into()), ..p.clone() };
        assert!(unlocks(&q, "it’s \"fine\""));
    }

    #[test]
    fn commit_always_asks_for_a_sentence() {
        let mut c = Ctx { site: "x.com", hour: 23, opens_today: 4, minutes_today: 0, longest_session: 15 };
        let mut saw = (false, false);
        for seed in 0..1000 {
            let p = pick(&[PromptKind::Commit], &c, None, seed);
            let sentence = p.sentence.expect("every commit prompt has a sentence");
            assert_eq!(p.text, SENTENCE_TEXT);
            saw.0 |= sentence == "I'm giving up sleep for x.com.";
            saw.1 |= sentence == "I've already been here 4 times today, and I'm back for more.";
        }
        assert_eq!(saw, (true, true));
        c.hour = 12;
        c.opens_today = 1;
        for seed in 0..1000 {
            let s = pick(&[PromptKind::Commit], &c, None, seed).sentence.unwrap();
            assert!(!s.contains("sleep") && !s.contains("already been here"), "{s}");
        }
        // the encouraging sentences come up alongside the pointed ones
        let picks: Vec<String> = (0..1000).map(|s| pick(&[PromptKind::Commit], &c, None, s).sentence.unwrap()).collect();
        assert!(picks.iter().any(|s| s == "I am destined to do great things."));
        assert!(picks.iter().any(|s| s == "I could close this tab, but I'm choosing x.com instead."));
        let free = Prompt { kind: PromptKind::Reflect, text: "Hi".into(), sentence: None };
        assert!(unlocks(&free, ""));
    }
}
