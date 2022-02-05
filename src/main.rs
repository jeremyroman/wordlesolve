use rand::{thread_rng, seq::SliceRandom};
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead};
use std::str;

fn to_letter_mask(c: u8) -> u32 {
    1 << (c - ('a' as u8))
}

#[derive(Debug, Clone, Copy)]
struct Word {
    bytes: [u8; 5],
    letters: u32,
}

impl Word {
    fn new(text: &str) -> Self {
        let mut bytes = [0; 5];
        bytes.copy_from_slice(text.as_bytes());
        let mut letters: u32 = 0;
        for b in bytes { letters |= to_letter_mask(b) }
        Self { bytes, letters }
    }
}

impl fmt::Display for Word {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let s = unsafe { str::from_utf8_unchecked(&self.bytes) };
        formatter.write_str(s)
    }
}

#[derive(Debug, Clone)]
struct Pattern {
    // Letters known to be present.
    positive_letters: u32,

    // Letters known to be absent.
    negative_letters: u32,

    // Letter masks per character.
    per_char: [u32; 5]
}

impl Pattern {
    fn new() -> Self {
        Self { positive_letters: 0, negative_letters: 0, per_char: [(1 << 26) - 1; 5] }
    }

    fn matches(&self, word: Word) -> bool {
        if word.letters & self.positive_letters != self.positive_letters { return false }
        if word.letters & self.negative_letters != 0 { return false }
        word.bytes.iter().zip(self.per_char.iter()).all(|(&w, &m)| (m & to_letter_mask(w)) != 0)
    }

    fn refine(&mut self, word: Word, Outcome(letter_outcomes): Outcome) {
        for i in 0..5 {
            let m = to_letter_mask(word.bytes[i]);
            match letter_outcomes[i] {
                LetterOutcome::Nowhere => {
                    self.negative_letters |= m;
                    for x in self.per_char.iter_mut() { *x &= !m; }
                },
                LetterOutcome::Elsewhere => {
                    self.positive_letters |= m;
                    self.per_char[i] &= !m;
                },
                LetterOutcome::Here => {
                    self.positive_letters |= m;
                    self.per_char[i] = m;
                },
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum LetterOutcome { Nowhere, Elsewhere, Here }

impl fmt::Display for LetterOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(match self {
            LetterOutcome::Here => "🟩",
            LetterOutcome::Elsewhere => "🟨",
            LetterOutcome::Nowhere => "⬜",
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Outcome([LetterOutcome; 5]);

impl Outcome {
    fn compare(goal: Word, guess: Word) -> Self {
        let mut result = [LetterOutcome::Nowhere; 5];
        for i in 0..5 {
            result[i] = if goal.bytes[i] == guess.bytes[i] {
                LetterOutcome::Here
            } else if goal.bytes.contains(&guess.bytes[i]) {
                LetterOutcome::Elsewhere
            } else {
                LetterOutcome::Nowhere
            }
        }
        Self(result)
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        for o in self.0 { o.fmt(formatter)? }
        Ok(())
    }
}

fn recommended_guess(pattern: &Pattern, goals: &Vec<Word>, dict: &Vec<Word>) -> (Word, i32) {
    let best_from = |dict: &Vec<Word>| -> (Word, i32) {
        let mut n = 0;
        dict.iter().map(|&guess| {
            n += 1;
            if n % 100 == 0 { eprint!("."); }
            let min_confidence = goals.iter().map(|&goal| -> i32 {
                let outcome = Outcome::compare(goal, guess);
                let mut hypothetical_pattern = pattern.clone();
                hypothetical_pattern.refine(guess, outcome);
                -goals.iter().fold(0, |c, &g| c + hypothetical_pattern.matches(g) as i32)
            }).min().unwrap();
            (guess, min_confidence)
        }).max_by_key(|p| p.1).unwrap()
    };
    let (bd, bdc) = best_from(dict);
    let (bg, bgc) = best_from(goals);
    assert!(bdc >= bgc);
    if bgc+1 >= bdc { (bg, bgc) } else { (bd, bdc) }
}

fn read_dict(filename: &str) -> io::Result<Vec<Word>> {
    let file = File::open(filename)?;
    let mut dict = Vec::new();
    for line_result in io::BufReader::new(file).lines() {
        let line = line_result?;
        if line.len() != 5 || line.chars().any(|c| !c.is_ascii_lowercase()) {
            Err(io::Error::new(io::ErrorKind::InvalidData, "malformed word"))?;
        }

        dict.push(Word::new(&line));
    }
    Ok(dict)
}

fn main() -> io::Result<()> {
    let mut goals = read_dict("goals.txt")?;
    let mut dict = read_dict("extra.txt")?;
    dict.extend(&goals);

    goals.shuffle(&mut thread_rng());
    dict.shuffle(&mut thread_rng());

    let args: Vec<String> = env::args().collect();
    let goal = Word::new(&args[1]);
    let mut pattern = Pattern::new();
    let mut buf = String::new();
    let stdin = io::stdin();
    loop {
        println!("pattern is {:?}", pattern);
        goals.retain(|w| pattern.matches(*w));
        println!("  {} matching goal words", goals.len());
        if goals.len() <= 20 {
            for g in &goals { println!("  {}", g); }
        }
        if goals.len() < 1000 {
            let (recommended, confidence) = recommended_guess(&pattern, &goals, &dict);
            println!("recommended guess is {} (at most {} possible words)", recommended, -confidence);
        }

        buf.clear();
        let length = stdin.read_line(&mut buf)?;
        if length != 6 || buf[0..5].chars().any(|c| !c.is_ascii_lowercase()) {
            println!("invalid");
            continue;
        }
        let guess = Word::new(&buf[0..5]);

        println!("guess matches pattern? {}", pattern.matches(guess));

        let outcome = Outcome::compare(goal, guess);
        println!("outcome is {}", outcome);
        pattern.refine(guess, outcome);
    }
}
