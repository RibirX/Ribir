use std::{
  collections::{HashMap, HashSet},
  io::{self, BufRead},
  sync::OnceLock,
};

use rand::prelude::*;
use ribir::prelude::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CharHint {
  Correct,
  Wrong,
  WrongPosition,
}

pub enum GameStatus {
  Continue,
  Lost(String),
  Win,
}

impl GameStatus {
  pub fn state_message(&self) -> String {
    match self {
      GameStatus::Continue => "".into(),
      GameStatus::Lost(word) => format!("Lost, the word is {}", word),
      GameStatus::Win => "Win!".into(),
    }
  }
}

#[derive(Debug, Query, Clone, Copy)]
pub struct WordleChar {
  pub char: char,
  pub hint: Option<CharHint>,
}

impl WordleChar {
  pub fn char(&self) -> char { self.char }

  pub fn hint(&self) -> Option<CharHint> { self.hint }
}

#[derive(Clone)]
pub struct WordleGuess(Vec<WordleChar>);

impl WordleGuess {
  pub fn iter(&self) -> impl Iterator<Item = &WordleChar> { self.0.iter() }

  pub fn char_hint(&self, idx: usize) -> &WordleChar { &self.0[idx] }
}

#[derive(Debug, Clone, Copy)]
pub enum InvalidInput {
  NotFinished,
  InvalidWorld,
}

impl InvalidInput {
  pub fn message(&self) -> &'static str {
    match self {
      InvalidInput::NotFinished => "Please input more characters",
      InvalidInput::InvalidWorld => "Invalid word",
    }
  }
}

pub struct WordleGuessing {
  disable: bool,
  max_len: usize,
  word: String,
}

impl WordleGuessing {
  pub fn new(max_len: usize) -> Self { Self { max_len, word: "".into(), disable: false } }

  pub fn word(&self) -> &str { self.word.as_str() }

  pub fn enter_char(&mut self, c: char) {
    if self.disable {
      return;
    }
    if self.word.len() < self.max_len && c.is_alphabetic() {
      self.word.push(c.to_uppercase().next().unwrap());
    }
  }

  pub fn delete_back_char(&mut self) {
    if !self.disable {
      self.word.pop();
    }
  }
}

#[derive(Query)]
pub struct Wordle {
  word: String,
  max_rounds: usize,
  give_up: bool,
  char_hints: HashMap<char, CharHint>,
  pub guessing: WordleGuessing,
  pub guesses: Vec<WordleGuess>,
}

impl Wordle {
  pub fn new(max_rounds: usize, word_len: usize) -> Self {
    loop {
      let idx = random::<usize>() % Self::word_dict().len();
      let word = Self::word_dict()
        .iter()
        .nth(idx)
        .cloned()
        .unwrap();
      if word.len() == word_len {
        return Self {
          word,
          give_up: false,
          guessing: WordleGuessing::new(word_len),
          char_hints: HashMap::new(),
          guesses: vec![],
          max_rounds,
        };
      }
    }
  }

  pub fn reset(&mut self) { *self = Self::new(self.max_rounds, self.word.len()); }

  pub fn give_up(&mut self) -> GameStatus {
    self.guessing.disable = true;
    GameStatus::Lost(self.word.clone())
  }

  pub fn max_rounds(&self) -> usize { self.max_rounds }

  pub fn len_hint(&self) -> usize { self.word.len() }

  pub fn guess(&mut self) -> Result<GameStatus, InvalidInput> {
    if self.guessing.disable {
      return Ok(self.status());
    }

    let word = &mut self.guessing.word;
    if word.len() < self.word.len() {
      Err(InvalidInput::NotFinished)
    } else if !Self::word_dict().contains(word) {
      Err(InvalidInput::InvalidWorld)
    } else {
      let word = std::mem::take(word);
      let guess = self.judge(&word);
      self.guesses.push(guess);

      let status = self.status();
      match status {
        GameStatus::Win => self.guessing.disable = true,
        GameStatus::Lost(_) => self.guessing.disable = true,
        _ => {}
      }
      Ok(status)
    }
  }

  fn is_win(&self) -> bool {
    self.guesses.last().map_or(false, |g| {
      g.0
        .iter()
        .all(|c| c.hint() == Some(CharHint::Correct))
    })
  }

  fn status(&self) -> GameStatus {
    if self.is_win() {
      GameStatus::Win
    } else if self.left_chances() == 0 || self.give_up {
      GameStatus::Lost(self.word.clone())
    } else {
      GameStatus::Continue
    }
  }

  pub fn left_chances(&self) -> usize { self.max_rounds - self.guesses.len() }

  fn judge(&mut self, word: &str) -> WordleGuess {
    let guess = WordleGuess(
      word
        .chars()
        .enumerate()
        .map(|(i, c)| {
          let status = if Some(c) == self.word.chars().nth(i) {
            CharHint::Correct
          } else if self.word.contains(c) {
            CharHint::WrongPosition
          } else {
            CharHint::Wrong
          };
          WordleChar { char: c, hint: Some(status) }
        })
        .collect::<Vec<_>>(),
    );
    for ele in guess.iter() {
      if ele.hint() == Some(CharHint::WrongPosition) {
        self
          .char_hints
          .entry(ele.char())
          .or_insert(CharHint::WrongPosition);
      } else {
        self
          .char_hints
          .insert(ele.char(), ele.hint().unwrap());
      }
    }
    guess
  }

  pub fn key_hint(&self, c: char) -> Option<CharHint> { self.char_hints.get(&c).copied() }

  fn word_dict() -> &'static HashSet<String> {
    static DICT: OnceLock<HashSet<String>> = OnceLock::new();
    DICT.get_or_init(|| {
      let mut dict = HashSet::new();
      io::BufReader::new(io::Cursor::new(include_str!("./dict.txt")))
        .lines()
        .filter(|l| l.is_ok())
        .for_each(|l| {
          l.unwrap().split(',').for_each(|w| {
            let w = w.trim();
            if !w.is_empty() {
              dict.insert(w.trim().to_uppercase());
            }
          })
        });
      dict
    })
  }
}
