/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

//! Declarative command scopes. Native editing gets first refusal; the nearest
//! matching scope owns an unconsumed chord, including when disabled.
//! 声明式快捷键作用域；原生编辑优先，最近的声明独占尚未消费的按键。
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Shortcut {
    #[schemars(regex(
        pattern = "^([a-z0-9]|escape|enter|delete|space|up|down|left|right|home|end|pageup|pagedown|f([1-9]|1[0-2]))$"
    ))]
    pub key: String,
    #[schemars(length(max = 4))]
    pub modifiers: Vec<Modifier>,
    #[schemars(length(min = 1, max = 128))]
    pub input: String,
    pub enabled: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    Control,
    Primary,
    Alt,
    Shift,
    Super,
}
/// Physical modifiers after the presenter resolves Primary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Modifiers {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}
impl Shortcut {
    pub fn physical(&self, primary_is_super: bool) -> Modifiers {
        let mut result = Modifiers::default();
        for modifier in &self.modifiers {
            match modifier {
                Modifier::Control => result.control = true,
                Modifier::Primary if primary_is_super => result.super_key = true,
                Modifier::Primary => result.control = true,
                Modifier::Alt => result.alt = true,
                Modifier::Shift => result.shift = true,
                Modifier::Super => result.super_key = true,
            }
        }
        result
    }
    pub fn matches(&self, key: &str, modifiers: Modifiers, primary_is_super: bool) -> bool {
        self.key == key && self.physical(primary_is_super) == modifiers
    }
}
pub fn validate(shortcuts: &[Shortcut], primary_is_super: bool) -> Result<(), String> {
    if shortcuts.len() > 32 {
        return Err("shortcut scope limit".into());
    }
    let mut chords = BTreeSet::new();
    for shortcut in shortcuts {
        let key = shortcut.key.as_str();
        let printable = key == "space"
            || key.len() == 1
                && key
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
        let named = matches!(
            key,
            "escape"
                | "enter"
                | "delete"
                | "space"
                | "up"
                | "down"
                | "left"
                | "right"
                | "home"
                | "end"
                | "pageup"
                | "pagedown"
        ) || key
            .strip_prefix('f')
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=12).contains(&n) && key == format!("f{n}"));
        let physical = shortcut.physical(primary_is_super);
        let unique_modifiers = shortcut
            .modifiers
            .iter()
            .enumerate()
            .all(|(i, m)| !shortcut.modifiers[..i].contains(m));
        if (!printable && !named)
            || shortcut.input.is_empty()
            || shortcut.input.len() > 128
            || shortcut.modifiers.len() > 4
            || !unique_modifiers
            || (printable && !physical.control && !physical.alt && !physical.super_key)
            || !chords.insert((key, physical))
        {
            return Err("invalid or duplicate shortcut chord".into());
        }
    }
    Ok(())
}
/// Merge command inputs with ordinary buttons, never with a typed native input.
/// The final component input table keeps the existing total bound.
pub fn inputs(
    shortcuts: &[Shortcut],
    inputs: &mut BTreeMap<String, serde_json::Value>,
) -> Result<(), String> {
    for shortcut in shortcuts.iter().filter(|shortcut| shortcut.enabled) {
        if let Some(schema) = inputs.get(&shortcut.input)
            && schema != &serde_json::Value::Bool(true)
        {
            return Err("shortcut collides with typed native input".into());
        }
        inputs.insert(shortcut.input.clone(), serde_json::Value::Bool(true));
        if inputs.len() > 64 {
            return Err("scene input limit".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn shortcut(key: &str, modifiers: Vec<Modifier>) -> Shortcut {
        Shortcut {
            key: key.into(),
            modifiers,
            input: "command".into(),
            enabled: true,
        }
    }
    #[test]
    fn physical_chords_are_unambiguous_and_leave_typing_native() {
        assert!(validate(&[shortcut("b", vec![])], false).is_err());
        assert!(validate(&[shortcut("b", vec![Modifier::Shift])], false).is_err());
        let primary = shortcut("b", vec![Modifier::Primary]);
        let control = shortcut("b", vec![Modifier::Control]);
        assert!(validate(&[primary.clone(), control.clone()], false).is_err());
        assert!(validate(&[primary, control], true).is_ok());
        assert!(validate(&[shortcut("escape", vec![])], false).is_ok());
        assert!(validate(&[shortcut("space", vec![])], false).is_err());
        assert!(validate(&[shortcut("f01", vec![])], false).is_err());
        assert!(
            validate(
                &[shortcut("b", vec![Modifier::Control, Modifier::Control])],
                false
            )
            .is_err()
        );
    }
    #[test]
    fn commands_merge_only_with_buttons_and_keep_the_input_bound() {
        let command = shortcut("b", vec![Modifier::Primary]);
        let mut table = BTreeMap::from([("command".into(), json!(true))]);
        inputs(std::slice::from_ref(&command), &mut table).unwrap();
        assert_eq!(table.len(), 1);
        table.insert("command".into(), json!({"type":"object"}));
        assert!(inputs(std::slice::from_ref(&command), &mut table).is_err());
        let mut table = (0..64)
            .map(|n| (format!("input-{n}"), json!(true)))
            .collect();
        assert!(inputs(&[command], &mut table).is_err());
    }
}
