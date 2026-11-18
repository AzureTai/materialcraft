//! Node creation shortcuts (Preferences ▸ Shortcuts): a key with modifiers that adds a library
//! entry under the pointer in the graph. The text of a shortcut is the menus' form, "Ctrl+Shift+B",
//! where Ctrl is Command on macOS

use egui::{Key, Modifiers};

/// The library categories whose entries can have a creation shortcut: the atomic nodes and the graph items
pub(crate) const CATEGORIES: [&str; 2] = ["Atomic nodes", "Graph items"];

/// The text of a key pressed with `modifiers`, or `None` for a key that cannot start a shortcut
pub(crate) fn text(modifiers: Modifiers, key: Key) -> Option<String> {
    // Plain Escape, Tab and Enter keep their usual work: closing, moving focus and confirming
    if modifiers.is_none() && matches!(key, Key::Escape | Key::Tab | Key::Enter) {
        return None;
    }

    let mut parts = Vec::new();
    if modifiers.command {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    parts.push(key.name());
    Some(parts.join("+"))
}

/// The modifiers and key a shortcut text names
pub(crate) fn parse(text: &str) -> Option<(Modifiers, Key)> {
    let mut modifiers = Modifiers::NONE;
    let mut key = None;
    for part in text.split('+') {
        match part {
            "Ctrl" | "Cmd" => modifiers.command = true,
            "Alt" => modifiers.alt = true,
            "Shift" => modifiers.shift = true,
            name if key.is_none() => key = Some(Key::from_name(name)?),
            _ => return None,
        }
    }
    key.map(|key| (modifiers, key))
}

/// The entry whose shortcut was pressed this frame, if any
pub(crate) fn pressed(shortcuts: &std::collections::BTreeMap<String, String>, input: &egui::InputState) -> Option<String> {
    shortcuts.iter().find_map(|(entry, text)| {
        let (modifiers, key) = parse(text)?;
        (input.modifiers.matches_logically(modifiers) && input.modifiers.shift == modifiers.shift && input.key_pressed(key)).then(|| entry.clone())
    })
}

/// The app command a shortcut text already starts, if any
pub(crate) fn taken_by(text: &str) -> Option<&'static str> {
    let wanted = parse(text)?;
    crate::menus::shortcut_table().into_iter().find(|(_, shortcut)| parse(shortcut) == Some(wanted)).map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shortcut_reads_back_as_the_keys_that_made_it() {
        let both = Modifiers { command: true, shift: true, ..Modifiers::NONE };
        assert_eq!(text(both, Key::B).as_deref(), Some("Ctrl+Shift+B"));
        assert_eq!(parse("Ctrl+Shift+B"), Some((both, Key::B)));
        assert_eq!(text(Modifiers::ALT, Key::Num1).and_then(|text| parse(&text)), Some((Modifiers::ALT, Key::Num1)));

        assert_eq!(text(Modifiers::NONE, Key::Escape), None);
        assert_eq!(parse("Ctrl+Nope"), None);
        assert_eq!(parse("Ctrl+B+C"), None);
    }

    #[test]
    fn a_shortcut_the_app_already_uses_is_named() {
        assert_eq!(taken_by("Ctrl+S"), Some("file.save"));
        assert_eq!(taken_by("Ctrl+Shift+B"), None);
    }
}
