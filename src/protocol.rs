use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    StartRun {
        #[serde(default = "default_character")]
        character: String,
        #[serde(default)]
        ascension: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        seed: Option<String>,
        #[serde(default = "default_lang")]
        lang: String,
    },
    Action {
        action: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<HashMap<String, Value>>,
    },
    GetMap,
    LoadSave {
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        json: Option<String>,
        #[serde(default = "default_lang")]
        lang: String,
    },
    Quit {
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
}

fn default_character() -> String {
    "Necrobinder".to_string()
}

fn default_lang() -> String {
    "en".to_string()
}

impl Command {
    pub fn action(name: impl Into<String>, args: Option<HashMap<String, Value>>) -> Self {
        Self::Action {
            action: name.into(),
            args,
        }
    }

    pub fn play_card(card_index: usize, target_index: Option<usize>) -> Self {
        let mut args = HashMap::new();
        args.insert("card_index".to_string(), Value::from(card_index as i64));
        if let Some(ti) = target_index {
            args.insert("target_index".to_string(), Value::from(ti as i64));
        }
        Self::action("play_card", Some(args))
    }

    pub fn end_turn() -> Self {
        Self::action("end_turn", None)
    }

    pub fn select_map_node(col: i32, row: i32) -> Self {
        let mut args = HashMap::new();
        args.insert("col".to_string(), Value::from(col));
        args.insert("row".to_string(), Value::from(row));
        Self::action("select_map_node", Some(args))
    }

    pub fn select_card_reward(card_index: usize) -> Self {
        let mut args = HashMap::new();
        args.insert("card_index".to_string(), Value::from(card_index as i64));
        Self::action("select_card_reward", Some(args))
    }

    pub fn skip_card_reward() -> Self {
        Self::action("skip_card_reward", None)
    }

    pub fn choose_option(option_index: usize) -> Self {
        let mut args = HashMap::new();
        args.insert("option_index".to_string(), Value::from(option_index as i64));
        Self::action("choose_option", Some(args))
    }

    pub fn select_cards(indices: &str) -> Self {
        let mut args = HashMap::new();
        args.insert("indices".to_string(), Value::String(indices.to_string()));
        Self::action("select_cards", Some(args))
    }

    pub fn buy_card(card_index: usize) -> Self {
        let mut args = HashMap::new();
        args.insert("card_index".to_string(), Value::from(card_index as i64));
        Self::action("buy_card", Some(args))
    }

    pub fn remove_card() -> Self {
        Self::action("remove_card", None)
    }

    pub fn leave_room() -> Self {
        Self::action("leave_room", None)
    }

    pub fn proceed() -> Self {
        Self::action("proceed", None)
    }
}

/// Dynamic text representation which can be a plain string or a bilingual map {"en": "...", "zh": "..."}.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocalizedString {
    Plain(String),
    Bilingual {
        #[serde(default)]
        en: Option<String>,
        #[serde(default)]
        zh: Option<String>,
    },
}

impl LocalizedString {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Plain(s) => s.as_str(),
            Self::Bilingual { en, zh } => {
                if let Some(e) = en {
                    e.as_str()
                } else if let Some(z) = zh {
                    z.as_str()
                } else {
                    ""
                }
            }
        }
    }
}

impl Default for LocalizedString {
    fn default() -> Self {
        Self::Plain(String::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CardStats {
    #[serde(default)]
    pub damage: Option<i32>,
    #[serde(default)]
    pub block: Option<i32>,
    #[serde(default)]
    pub magic: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CardInfo {
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub name: LocalizedString,
    #[serde(default)]
    pub cost: i32,
    #[serde(rename = "type", default)]
    pub card_type: String,
    #[serde(default)]
    pub can_play: bool,
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub stats: Option<CardStats>,
    #[serde(default)]
    pub keywords: Option<Vec<String>>,
    #[serde(default)]
    pub is_stocked: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnemyIntent {
    #[serde(rename = "type", default)]
    pub intent_type: String,
    #[serde(default)]
    pub damage: Option<i32>,
    #[serde(default)]
    pub hits: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnemyInfo {
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub name: LocalizedString,
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub max_hp: i32,
    #[serde(default)]
    pub block: i32,
    #[serde(default)]
    pub intents: Option<Vec<EnemyIntent>>,
    #[serde(default)]
    pub intends_attack: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OstyState {
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub max_hp: i32,
    #[serde(default)]
    pub block: i32,
    #[serde(default)]
    pub alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerState {
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub max_hp: i32,
    #[serde(default)]
    pub gold: i32,
    #[serde(default)]
    pub energy: i32,
    #[serde(default)]
    pub block: i32,
    #[serde(default)]
    pub deck_size: usize,
    #[serde(default)]
    pub deck: Option<Vec<CardInfo>>,
    #[serde(default)]
    pub potions: Option<Vec<PotionInfo>>,
    #[serde(default)]
    pub relics: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PotionInfo {
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub name: LocalizedString,
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub can_use: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MapChoice {
    #[serde(default)]
    pub col: i32,
    #[serde(default)]
    pub row: i32,
    #[serde(rename = "type", default)]
    pub room_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventOption {
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub title: LocalizedString,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub vars: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameContext {
    #[serde(default)]
    pub act: i32,
    #[serde(default)]
    pub floor: i32,
    #[serde(default)]
    pub room_type: Option<String>,
    #[serde(default)]
    pub boss: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameState {
    #[serde(rename = "type", default)]
    pub msg_type: String,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub player: Option<PlayerState>,
    #[serde(default)]
    pub context: Option<GameContext>,
    #[serde(default)]
    pub hand: Option<Vec<CardInfo>>,
    #[serde(default)]
    pub enemies: Option<Vec<EnemyInfo>>,
    #[serde(default)]
    pub osty: Option<OstyState>,
    #[serde(default)]
    pub energy: Option<i32>,
    #[serde(default)]
    pub round: Option<i32>,
    #[serde(default)]
    pub choices: Option<Vec<MapChoice>>,
    #[serde(default)]
    pub cards: Option<Vec<CardInfo>>,
    #[serde(default)]
    pub options: Option<Vec<EventOption>>,
    #[serde(default)]
    pub potions: Option<Vec<PotionInfo>>,
    #[serde(default)]
    pub card_removal_cost: Option<i32>,
    #[serde(default)]
    pub min_select: Option<usize>,
    #[serde(default)]
    pub max_select: Option<usize>,
    #[serde(default)]
    pub victory: Option<bool>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

impl GameState {
    pub fn is_ready(&self) -> bool {
        self.msg_type == "ready"
    }

    pub fn is_error(&self) -> bool {
        self.msg_type == "error"
    }

    pub fn decision_name(&self) -> &str {
        if let Some(ref d) = self.decision {
            d.as_str()
        } else {
            self.msg_type.as_str()
        }
    }
}
