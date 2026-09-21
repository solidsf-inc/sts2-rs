use crate::protocol::{
    CardInfo, CardStats, Command, EnemyInfo, EnemyIntent, GameContext, GameState, LocalizedString,
    MapChoice, OstyState, PlayerState,
};
use anyhow::{bail, Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command as OsCommand, Stdio};

pub trait SimulatorDriver {
    fn send(&mut self, cmd: &Command) -> Result<GameState>;
    fn close(&mut self) -> Result<()>;
}

pub struct SubprocessDriver {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
}

impl SubprocessDriver {
    pub fn new(repo_root: &Path) -> Result<Self> {
        // Search candidate roots: explicit path, env var, current dir, parent, ../sts2-cli
        let mut candidates = Vec::new();
        if let Ok(env_root) = std::env::var("STS2_ROOT") {
            candidates.push(PathBuf::from(env_root));
        }
        candidates.push(repo_root.to_path_buf());
        candidates.push(PathBuf::from("."));
        candidates.push(PathBuf::from(".."));
        candidates.push(PathBuf::from("../sts2-cli"));

        let mut resolved_root = None;
        for cand in candidates {
            if let Ok(canonical) = std::fs::canonicalize(&cand) {
                if canonical
                    .join("src")
                    .join("Sts2Headless")
                    .join("Sts2Headless.csproj")
                    .exists()
                {
                    resolved_root = Some(canonical);
                    break;
                }
            }
        }

        let abs_root = match resolved_root {
            Some(r) => r,
            None => {
                bail!("Could not find Sts2Headless project. Set STS2_ROOT environment variable or pass --root /path/to/sts2-cli");
            }
        };

        let lib_dll = abs_root.join("lib").join("sts2.dll");
        if !lib_dll.exists() {
            bail!(
                "Game DLL not found at {}. Live mode requires Slay the Spire 2 DLLs in lib/",
                lib_dll.display()
            );
        }

        let project_path = abs_root
            .join("src")
            .join("Sts2Headless")
            .join("Sts2Headless.csproj");

        let dotnet_bin = find_dotnet();

        let mut cmd = OsCommand::new(&dotnet_bin);
        cmd.args(["run", "--no-build", "--project"])
            .arg(&project_path)
            .current_dir(&abs_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn dotnet process: {}", dotnet_bin))?;

        let stdin = child
            .stdin
            .take()
            .context("Failed to open stdin for simulator")?;
        let stdout = child
            .stdout
            .take()
            .context("Failed to open stdout for simulator")?;
        let mut reader = BufReader::new(stdout);

        // Read initial ready packet
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let ready_state: GameState = serde_json::from_str(&line)
            .with_context(|| format!("Invalid ready state: {}", line))?;

        if !ready_state.is_ready() {
            bail!("Expected ready message, got: {:?}", ready_state);
        }

        Ok(Self {
            child,
            stdin,
            reader,
        })
    }
}

impl SimulatorDriver for SubprocessDriver {
    fn send(&mut self, cmd: &Command) -> Result<GameState> {
        let json_line = serde_json::to_string(cmd)?;
        writeln!(self.stdin, "{}", json_line)?;
        self.stdin.flush()?;

        let mut line = String::new();
        while self.reader.read_line(&mut line)? > 0 {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                line.clear();
                continue;
            }
            if trimmed.starts_with('{') {
                let state: GameState = serde_json::from_str(trimmed)
                    .with_context(|| format!("Failed to parse JSON response: {}", trimmed))?;
                return Ok(state);
            }
            line.clear();
        }

        bail!("EOF received from simulator process")
    }

    fn close(&mut self) -> Result<()> {
        let _ = writeln!(self.stdin, "{{\"cmd\":\"quit\"}}");
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
        Ok(())
    }
}

fn find_dotnet() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{}/.dotnet-arm64/dotnet", home),
        format!("{}/.dotnet/dotnet", home),
        "dotnet".to_string(),
    ];
    for p in &candidates {
        if Path::new(p).exists() {
            return p.clone();
        }
    }
    "dotnet".to_string()
}

/// A high-fidelity mock simulator harness for testing and verification
/// when the licensed Steam game files are absent.
pub struct MockDriver {
    pub floor: i32,
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub player_gold: i32,
    pub enemy_hp: i32,
    pub combat_round: i32,
    pub osty_hp: i32,
    pub current_phase: String,
}

impl Default for MockDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl MockDriver {
    pub fn new() -> Self {
        Self {
            floor: 1,
            player_hp: 75,
            player_max_hp: 75,
            player_gold: 99,
            enemy_hp: 42,
            combat_round: 1,
            osty_hp: 5,
            current_phase: "ready".to_string(),
        }
    }
}

impl SimulatorDriver for MockDriver {
    fn send(&mut self, cmd: &Command) -> Result<GameState> {
        match cmd {
            Command::StartRun { .. } => {
                self.current_phase = "map_select".to_string();
                self.floor = 1;
                Ok(GameState {
                    msg_type: "decision".to_string(),
                    decision: Some("map_select".to_string()),
                    context: Some(GameContext {
                        act: 1,
                        floor: self.floor,
                        room_type: Some("Map".to_string()),
                        boss: None,
                    }),
                    player: Some(PlayerState {
                        hp: self.player_hp,
                        max_hp: self.player_max_hp,
                        gold: self.player_gold,
                        energy: 3,
                        block: 0,
                        deck_size: 10,
                        ..Default::default()
                    }),
                    choices: Some(vec![
                        MapChoice {
                            col: 0,
                            row: 1,
                            room_type: "Monster".to_string(),
                        },
                        MapChoice {
                            col: 1,
                            row: 1,
                            room_type: "Treasure".to_string(),
                        },
                    ]),
                    ..Default::default()
                })
            }
            Command::Action { action, .. } => {
                match action.as_str() {
                    "select_map_node" => {
                        self.current_phase = "combat_play".to_string();
                        self.combat_round = 1;
                        self.enemy_hp = 42;
                        Ok(GameState {
                            msg_type: "decision".to_string(),
                            decision: Some("combat_play".to_string()),
                            round: Some(self.combat_round),
                            energy: Some(3),
                            player: Some(PlayerState {
                                hp: self.player_hp,
                                max_hp: self.player_max_hp,
                                gold: self.player_gold,
                                energy: 3,
                                block: 0,
                                deck_size: 10,
                                ..Default::default()
                            }),
                            osty: Some(OstyState {
                                hp: self.osty_hp,
                                max_hp: 10,
                                block: 0,
                                alive: true,
                            }),
                            enemies: Some(vec![EnemyInfo {
                                index: 0,
                                name: LocalizedString::Plain("Cultist".to_string()),
                                hp: self.enemy_hp,
                                max_hp: 48,
                                block: 0,
                                intents: Some(vec![EnemyIntent {
                                    intent_type: "Attack".to_string(),
                                    damage: Some(6),
                                    hits: Some(1),
                                }]),
                                intends_attack: Some(true),
                            }]),
                            hand: Some(vec![
                                CardInfo {
                                    index: 0,
                                    name: LocalizedString::Plain("Flatten".to_string()),
                                    cost: 1,
                                    card_type: "Attack".to_string(),
                                    can_play: true,
                                    target_type: Some("AnyEnemy".to_string()),
                                    stats: Some(CardStats {
                                        damage: Some(18),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                },
                                CardInfo {
                                    index: 1,
                                    name: LocalizedString::Plain("Bodyguard".to_string()),
                                    cost: 1,
                                    card_type: "Skill".to_string(),
                                    can_play: true,
                                    target_type: Some("Self".to_string()),
                                    ..Default::default()
                                },
                                CardInfo {
                                    index: 2,
                                    name: LocalizedString::Plain("Defend".to_string()),
                                    cost: 1,
                                    card_type: "Skill".to_string(),
                                    can_play: true,
                                    target_type: Some("Self".to_string()),
                                    ..Default::default()
                                },
                            ]),
                            ..Default::default()
                        })
                    }
                    "play_card" => {
                        // Subtract enemy HP by card damage
                        self.enemy_hp -= 18;
                        if self.enemy_hp <= 0 {
                            // Enemy defeated! Move to card reward
                            self.current_phase = "card_reward".to_string();
                            self.player_gold += 25;
                            Ok(GameState {
                                msg_type: "decision".to_string(),
                                decision: Some("card_reward".to_string()),
                                player: Some(PlayerState {
                                    hp: self.player_hp,
                                    max_hp: self.player_max_hp,
                                    gold: self.player_gold,
                                    deck_size: 10,
                                    ..Default::default()
                                }),
                                cards: Some(vec![
                                    CardInfo {
                                        index: 0,
                                        name: LocalizedString::Plain("Calcify".to_string()),
                                        card_type: "Power".to_string(),
                                        ..Default::default()
                                    },
                                    CardInfo {
                                        index: 1,
                                        name: LocalizedString::Plain("Strike".to_string()),
                                        card_type: "Attack".to_string(),
                                        ..Default::default()
                                    },
                                ]),
                                ..Default::default()
                            })
                        } else {
                            // Enemy still alive
                            Ok(GameState {
                                msg_type: "decision".to_string(),
                                decision: Some("combat_play".to_string()),
                                round: Some(self.combat_round),
                                energy: Some(2),
                                player: Some(PlayerState {
                                    hp: self.player_hp,
                                    max_hp: self.player_max_hp,
                                    gold: self.player_gold,
                                    energy: 2,
                                    block: 5,
                                    deck_size: 10,
                                    ..Default::default()
                                }),
                                osty: Some(OstyState {
                                    hp: self.osty_hp,
                                    max_hp: 10,
                                    block: 0,
                                    alive: true,
                                }),
                                enemies: Some(vec![EnemyInfo {
                                    index: 0,
                                    name: LocalizedString::Plain("Cultist".to_string()),
                                    hp: self.enemy_hp,
                                    max_hp: 48,
                                    block: 0,
                                    intents: Some(vec![EnemyIntent {
                                        intent_type: "Attack".to_string(),
                                        damage: Some(6),
                                        hits: Some(1),
                                    }]),
                                    intends_attack: Some(true),
                                }]),
                                hand: Some(vec![CardInfo {
                                    index: 1,
                                    name: LocalizedString::Plain("Bodyguard".to_string()),
                                    cost: 1,
                                    card_type: "Skill".to_string(),
                                    can_play: true,
                                    target_type: Some("Self".to_string()),
                                    ..Default::default()
                                }]),
                                ..Default::default()
                            })
                        }
                    }
                    "select_card_reward" | "skip_card_reward" => {
                        self.floor += 1;
                        if self.floor >= 4 {
                            self.current_phase = "game_over".to_string();
                            Ok(GameState {
                                msg_type: "decision".to_string(),
                                decision: Some("game_over".to_string()),
                                victory: Some(true),
                                player: Some(PlayerState {
                                    hp: self.player_hp,
                                    max_hp: self.player_max_hp,
                                    gold: self.player_gold,
                                    deck_size: 11,
                                    ..Default::default()
                                }),
                                context: Some(GameContext {
                                    act: 1,
                                    floor: self.floor,
                                    room_type: Some("Boss".to_string()),
                                    boss: None,
                                }),
                                ..Default::default()
                            })
                        } else {
                            self.current_phase = "rest_site".to_string();
                            Ok(GameState {
                                msg_type: "decision".to_string(),
                                decision: Some("rest_site".to_string()),
                                player: Some(PlayerState {
                                    hp: self.player_hp,
                                    max_hp: self.player_max_hp,
                                    gold: self.player_gold,
                                    deck_size: 11,
                                    ..Default::default()
                                }),
                                context: Some(GameContext {
                                    act: 1,
                                    floor: self.floor,
                                    room_type: Some("RestSite".to_string()),
                                    boss: None,
                                }),
                                ..Default::default()
                            })
                        }
                    }
                    "choose_option" => {
                        // Rest site heal or smith
                        self.floor += 1;
                        self.current_phase = "shop".to_string();
                        Ok(GameState {
                            msg_type: "decision".to_string(),
                            decision: Some("shop".to_string()),
                            player: Some(PlayerState {
                                hp: self.player_hp,
                                max_hp: self.player_max_hp,
                                gold: self.player_gold,
                                deck_size: 11,
                                ..Default::default()
                            }),
                            card_removal_cost: Some(75),
                            cards: Some(vec![CardInfo {
                                index: 0,
                                name: LocalizedString::Plain("Unleash".to_string()),
                                cost: 50,
                                is_stocked: Some(true),
                                ..Default::default()
                            }]),
                            ..Default::default()
                        })
                    }
                    "buy_card" | "remove_card" | "leave_room" => {
                        // Move to Boss
                        self.floor += 1;
                        self.current_phase = "game_over".to_string();
                        Ok(GameState {
                            msg_type: "decision".to_string(),
                            decision: Some("game_over".to_string()),
                            victory: Some(true),
                            player: Some(PlayerState {
                                hp: self.player_hp,
                                max_hp: self.player_max_hp,
                                gold: self.player_gold,
                                deck_size: 12,
                                ..Default::default()
                            }),
                            context: Some(GameContext {
                                act: 1,
                                floor: self.floor,
                                room_type: Some("Boss".to_string()),
                                boss: None,
                            }),
                            ..Default::default()
                        })
                    }
                    _ => Ok(GameState {
                        msg_type: "decision".to_string(),
                        decision: Some("map_select".to_string()),
                        ..Default::default()
                    }),
                }
            }
            Command::Quit { .. } => Ok(GameState {
                msg_type: "quit_result".to_string(),
                ..Default::default()
            }),
            _ => Ok(GameState {
                msg_type: "ready".to_string(),
                version: Some("0.2.0".to_string()),
                ..Default::default()
            }),
        }
    }

    fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
