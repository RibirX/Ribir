use crate::events::{KeyCode, NamedKey, PhysicalKey, VirtualKey};

pub(crate) const SUPPORTED_LOGICAL_NAMED_KEYS: &[&str] = &[
  "Enter",
  "Tab",
  "Space",
  "Escape",
  "Backspace",
  "Delete",
  "ArrowUp",
  "ArrowDown",
  "ArrowLeft",
  "ArrowRight",
  "Home",
  "End",
  "PageUp",
  "PageDown",
];

pub(crate) const SUPPORTED_PHYSICAL_CODES: &[&str] = &[
  "Enter",
  "Tab",
  "Space",
  "Escape",
  "Backspace",
  "Delete",
  "ArrowUp",
  "ArrowDown",
  "ArrowLeft",
  "ArrowRight",
  "Home",
  "End",
  "PageUp",
  "PageDown",
  "KeyA",
  "KeyB",
  "KeyC",
  "KeyD",
  "KeyE",
  "KeyF",
  "KeyG",
  "KeyH",
  "KeyI",
  "KeyJ",
  "KeyK",
  "KeyL",
  "KeyM",
  "KeyN",
  "KeyO",
  "KeyP",
  "KeyQ",
  "KeyR",
  "KeyS",
  "KeyT",
  "KeyU",
  "KeyV",
  "KeyW",
  "KeyX",
  "KeyY",
  "KeyZ",
  "Digit0",
  "Digit1",
  "Digit2",
  "Digit3",
  "Digit4",
  "Digit5",
  "Digit6",
  "Digit7",
  "Digit8",
  "Digit9",
];

pub(crate) fn normalize_key_name(name: &str) -> String {
  name
    .trim()
    .chars()
    .filter(|c| c.is_ascii_alphanumeric())
    .map(|c| c.to_ascii_lowercase())
    .collect()
}

pub(crate) fn parse_key_code(name: &str) -> Option<KeyCode> {
  match normalize_key_name(name).as_str() {
    "enter" => Some(KeyCode::Enter),
    "tab" => Some(KeyCode::Tab),
    "space" => Some(KeyCode::Space),
    "escape" | "esc" => Some(KeyCode::Escape),
    "backspace" => Some(KeyCode::Backspace),
    "delete" => Some(KeyCode::Delete),
    "arrowup" => Some(KeyCode::ArrowUp),
    "arrowdown" => Some(KeyCode::ArrowDown),
    "arrowleft" => Some(KeyCode::ArrowLeft),
    "arrowright" => Some(KeyCode::ArrowRight),
    "home" => Some(KeyCode::Home),
    "end" => Some(KeyCode::End),
    "pageup" => Some(KeyCode::PageUp),
    "pagedown" => Some(KeyCode::PageDown),
    "digit0" => Some(KeyCode::Digit0),
    "digit1" => Some(KeyCode::Digit1),
    "digit2" => Some(KeyCode::Digit2),
    "digit3" => Some(KeyCode::Digit3),
    "digit4" => Some(KeyCode::Digit4),
    "digit5" => Some(KeyCode::Digit5),
    "digit6" => Some(KeyCode::Digit6),
    "digit7" => Some(KeyCode::Digit7),
    "digit8" => Some(KeyCode::Digit8),
    "digit9" => Some(KeyCode::Digit9),
    "keya" => Some(KeyCode::KeyA),
    "keyb" => Some(KeyCode::KeyB),
    "keyc" => Some(KeyCode::KeyC),
    "keyd" => Some(KeyCode::KeyD),
    "keye" => Some(KeyCode::KeyE),
    "keyf" => Some(KeyCode::KeyF),
    "keyg" => Some(KeyCode::KeyG),
    "keyh" => Some(KeyCode::KeyH),
    "keyi" => Some(KeyCode::KeyI),
    "keyj" => Some(KeyCode::KeyJ),
    "keyk" => Some(KeyCode::KeyK),
    "keyl" => Some(KeyCode::KeyL),
    "keym" => Some(KeyCode::KeyM),
    "keyn" => Some(KeyCode::KeyN),
    "keyo" => Some(KeyCode::KeyO),
    "keyp" => Some(KeyCode::KeyP),
    "keyq" => Some(KeyCode::KeyQ),
    "keyr" => Some(KeyCode::KeyR),
    "keys" => Some(KeyCode::KeyS),
    "keyt" => Some(KeyCode::KeyT),
    "keyu" => Some(KeyCode::KeyU),
    "keyv" => Some(KeyCode::KeyV),
    "keyw" => Some(KeyCode::KeyW),
    "keyx" => Some(KeyCode::KeyX),
    "keyy" => Some(KeyCode::KeyY),
    "keyz" => Some(KeyCode::KeyZ),
    _ => None,
  }
}

pub(crate) fn parse_virtual_key(key: &str) -> Option<VirtualKey> {
  if let Some(named) = parse_named_key(key) {
    return Some(VirtualKey::Named(named));
  }
  if key.chars().count() == 1 {
    return Some(VirtualKey::Character(key.into()));
  }
  None
}

pub(crate) fn derive_physical_key(key: &str) -> Option<PhysicalKey> {
  if let Some(code) = parse_key_code(key) {
    return Some(PhysicalKey::Code(code));
  }

  if key.chars().count() == 1 {
    let ch = key.chars().next()?;
    return char_to_key_code(ch).map(PhysicalKey::Code);
  }
  None
}

pub(crate) fn infer_receive_chars_from_key(key: &str) -> Option<String> {
  if key.chars().count() == 1 {
    return Some(key.to_string());
  }

  match normalize_key_name(key).as_str() {
    "space" | "spacebar" => Some(" ".to_string()),
    "tab" => Some("\t".to_string()),
    _ => None,
  }
}

pub(crate) fn closest_key_names<'a>(
  input: &str, candidates: &'a [&'a str], limit: usize,
) -> Vec<&'a str> {
  let normalized_input = normalize_key_name(input);
  let mut scored: Vec<(usize, bool, bool, &'a str)> = candidates
    .iter()
    .copied()
    .map(|candidate| {
      let normalized_candidate = normalize_key_name(candidate);
      (
        levenshtein_distance(&normalized_input, &normalized_candidate),
        normalized_candidate.starts_with(&normalized_input),
        normalized_candidate.contains(&normalized_input),
        candidate,
      )
    })
    .collect();

  scored.sort_by(|a, b| {
    a.0
      .cmp(&b.0)
      .then_with(|| b.1.cmp(&a.1))
      .then_with(|| b.2.cmp(&a.2))
  });

  scored
    .into_iter()
    .take(limit)
    .map(|(_, _, _, candidate)| candidate)
    .collect()
}

pub(crate) fn keyboard_key_error(key: &str) -> String {
  let closest = closest_key_names(key, SUPPORTED_LOGICAL_NAMED_KEYS, 5).join(", ");
  let supported = SUPPORTED_LOGICAL_NAMED_KEYS.join(", ");
  format!(
    "Unsupported keyboard_input key '{}'. Use W3C KeyboardEvent.key names (e.g. Enter, ArrowLeft) \
     or single-character keys (e.g. 'a', '1'). Closest matches: {}. Supported named keys: {}.",
    key, closest, supported
  )
}

pub(crate) fn keyboard_physical_key_error(physical_key: &str) -> String {
  let closest = closest_key_names(physical_key, SUPPORTED_PHYSICAL_CODES, 5).join(", ");
  let supported = SUPPORTED_PHYSICAL_CODES.join(", ");
  format!(
    "Unsupported keyboard_input physical_key '{}'. Use W3C KeyboardEvent.code names (e.g. KeyA, \
     Digit1, Enter, ArrowLeft). Closest matches: {}. Supported physical_key values: {}.",
    physical_key, closest, supported
  )
}

fn parse_named_key(name: &str) -> Option<NamedKey> {
  match normalize_key_name(name).as_str() {
    "enter" => Some(NamedKey::Enter),
    "tab" => Some(NamedKey::Tab),
    "space" | "spacebar" => Some(NamedKey::Space),
    "escape" | "esc" => Some(NamedKey::Escape),
    "backspace" => Some(NamedKey::Backspace),
    "delete" => Some(NamedKey::Delete),
    "arrowup" => Some(NamedKey::ArrowUp),
    "arrowdown" => Some(NamedKey::ArrowDown),
    "arrowleft" => Some(NamedKey::ArrowLeft),
    "arrowright" => Some(NamedKey::ArrowRight),
    "home" => Some(NamedKey::Home),
    "end" => Some(NamedKey::End),
    "pageup" => Some(NamedKey::PageUp),
    "pagedown" => Some(NamedKey::PageDown),
    _ => None,
  }
}

fn char_to_key_code(ch: char) -> Option<KeyCode> {
  match ch {
    'a' | 'A' => Some(KeyCode::KeyA),
    'b' | 'B' => Some(KeyCode::KeyB),
    'c' | 'C' => Some(KeyCode::KeyC),
    'd' | 'D' => Some(KeyCode::KeyD),
    'e' | 'E' => Some(KeyCode::KeyE),
    'f' | 'F' => Some(KeyCode::KeyF),
    'g' | 'G' => Some(KeyCode::KeyG),
    'h' | 'H' => Some(KeyCode::KeyH),
    'i' | 'I' => Some(KeyCode::KeyI),
    'j' | 'J' => Some(KeyCode::KeyJ),
    'k' | 'K' => Some(KeyCode::KeyK),
    'l' | 'L' => Some(KeyCode::KeyL),
    'm' | 'M' => Some(KeyCode::KeyM),
    'n' | 'N' => Some(KeyCode::KeyN),
    'o' | 'O' => Some(KeyCode::KeyO),
    'p' | 'P' => Some(KeyCode::KeyP),
    'q' | 'Q' => Some(KeyCode::KeyQ),
    'r' | 'R' => Some(KeyCode::KeyR),
    's' | 'S' => Some(KeyCode::KeyS),
    't' | 'T' => Some(KeyCode::KeyT),
    'u' | 'U' => Some(KeyCode::KeyU),
    'v' | 'V' => Some(KeyCode::KeyV),
    'w' | 'W' => Some(KeyCode::KeyW),
    'x' | 'X' => Some(KeyCode::KeyX),
    'y' | 'Y' => Some(KeyCode::KeyY),
    'z' | 'Z' => Some(KeyCode::KeyZ),
    '0' => Some(KeyCode::Digit0),
    '1' => Some(KeyCode::Digit1),
    '2' => Some(KeyCode::Digit2),
    '3' => Some(KeyCode::Digit3),
    '4' => Some(KeyCode::Digit4),
    '5' => Some(KeyCode::Digit5),
    '6' => Some(KeyCode::Digit6),
    '7' => Some(KeyCode::Digit7),
    '8' => Some(KeyCode::Digit8),
    '9' => Some(KeyCode::Digit9),
    _ => None,
  }
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
  let a_chars: Vec<char> = a.chars().collect();
  let b_chars: Vec<char> = b.chars().collect();
  let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
  let mut cur = vec![0; b_chars.len() + 1];

  for (i, &ca) in a_chars.iter().enumerate() {
    cur[0] = i + 1;
    for (j, &cb) in b_chars.iter().enumerate() {
      let cost = usize::from(ca != cb);
      cur[j + 1] = (prev[j + 1] + 1)
        .min(cur[j] + 1)
        .min(prev[j] + cost);
    }
    std::mem::swap(&mut prev, &mut cur);
  }

  prev[b_chars.len()]
}
