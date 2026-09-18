use std::path::{Path, PathBuf};
use yamake::model::GNode;

use crate::chords::bar_numbering::barcount_map_of_structure;
use crate::model::{Click, Clicks, ClicksDefinition, RawClick, RawClicksDefinition, Song};

/// Build node for `clicks.yml` that generates click times by linear interpolation
/// from a `ClickDef` predecessor.
pub struct ClickYml {
    pub path: PathBuf,
}

impl ClickYml {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

/// Compute the absolute beat number (1-based) from bar and beat-in-bar.
/// Uses 4 beats per bar: `(bar_number - 1) * 4 + beat_in_bar_number`.
fn absolute_beat(click: &Click) -> u32 {
    (click.bar_number - 1) * 4 + click.beat_in_bar_number
}

/// Resolve a `RawClicksDefinition` into a `ClicksDefinition` by looking up
/// `section_id` entries in the bar map derived from the song structure.
///
/// Returns an error string if a `section_id` cannot be found.
pub fn resolve(raw: &RawClicksDefinition, song: Option<&Song>) -> Result<ClicksDefinition, String> {
    let bar_map = song.map(|s| barcount_map_of_structure(&s.structure));

    let clicks = raw
        .clicks
        .iter()
        .map(|raw_click| resolve_click(raw_click, bar_map.as_ref()))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ClicksDefinition { clicks })
}

fn resolve_click(
    raw: &RawClick,
    bar_map: Option<&std::collections::HashMap<String, (Vec<i32>, i32)>>,
) -> Result<Click, String> {
    let bar_number = match (&raw.bar_number, &raw.section_id) {
        (Some(n), None) => *n,
        (None, Some(id)) => {
            let bmap = bar_map
                .ok_or_else(|| format!("section_id '{id}' used but no song structure available"))?;
            let (row_numbers, _) = bmap
                .get(id)
                .ok_or_else(|| format!("section_id '{id}' not found in song structure"))?;
            let first = row_numbers
                .first()
                .ok_or_else(|| format!("section '{id}' has no bars"))?;
            *first as u32
        }
        // These cases are validated at ClickDef::new time, but handle defensively
        (None, None) => {
            return Err("click entry has neither bar_number nor section_id".to_string());
        }
        (Some(_), Some(id)) => {
            return Err(format!(
                "click entry has both bar_number and section_id '{id}'"
            ));
        }
    };

    Ok(Click {
        bar_number,
        beat_in_bar_number: raw.beat_in_bar_number,
        time: raw.time,
        description: raw.description.clone(),
    })
}

/// Interpolate click times from a `ClicksDefinition`.
///
/// For each pair of consecutive Click entries, linearly interpolates
/// the beat times between them. For example, if bar 1 beat 1 is at 0.0s and
/// bar 2 beat 1 is at 2.0s, the 4 intermediate beats will be at 0.0, 0.5, 1.0, 1.5.
fn interpolate(def: &ClicksDefinition) -> Vec<f64> {
    if def.clicks.is_empty() {
        return vec![];
    }
    if def.clicks.len() == 1 {
        return vec![def.clicks[0].time];
    }

    let mut result = Vec::new();
    for window in def.clicks.windows(2) {
        let from = &window[0];
        let to = &window[1];
        let n_beats = absolute_beat(to) - absolute_beat(from);
        for i in 0..n_beats {
            let t = from.time + (to.time - from.time) * (i as f64) / (n_beats as f64);
            result.push(t);
        }
    }
    // Add the last click's time
    result.push(def.clicks.last().unwrap().time);
    result
}

impl GNode for ClickYml {
    fn tag(&self) -> String {
        "clicks.yml".to_string()
    }

    fn pathbuf(&self) -> PathBuf {
        self.path.clone()
    }

    fn build(&self, sandbox: &Path, predecessors: &[&(dyn GNode + Send + Sync)]) -> bool {
        // Find the ClickDef predecessor
        let click_defs: Vec<_> = predecessors
            .iter()
            .filter(|p| p.tag() == "clicks-def")
            .collect();

        if click_defs.len() != 1 {
            log::error!(
                "ClickYml {} expected 1 clicks-def predecessor, got {}",
                self.path.display(),
                click_defs.len()
            );
            return false;
        }

        // Read the RawClicksDefinition from the predecessor
        let def_path = sandbox.join(click_defs[0].pathbuf());
        let yaml = match std::fs::read_to_string(&def_path) {
            Ok(y) => y,
            Err(e) => {
                log::error!("Failed to read {}: {e}", def_path.display());
                return false;
            }
        };
        let raw_def: RawClicksDefinition = match serde_yaml::from_str(&yaml) {
            Ok(d) => d,
            Err(e) => {
                log::error!("Invalid clicks definition {}: {e}", def_path.display());
                return false;
            }
        };

        // Check if any entry uses section_id — if so, load the sibling song.yml
        let needs_song = raw_def.clicks.iter().any(|c| c.section_id.is_some());
        let song: Option<Song> = if needs_song {
            // clicks-def.yml lives at <song_dir>/clicks-def.yml; song.yml is in the same dir
            let song_yml_path = def_path
                .parent()
                .map(|p| p.join("song.yml"))
                .unwrap_or_else(|| PathBuf::from("song.yml"));
            match std::fs::read_to_string(&song_yml_path) {
                Ok(y) => match serde_yaml::from_str::<Song>(&y) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        log::error!(
                            "Failed to parse song.yml for section_id resolution at {}: {e}",
                            song_yml_path.display()
                        );
                        return false;
                    }
                },
                Err(e) => {
                    log::error!(
                        "Failed to read song.yml for section_id resolution at {}: {e}",
                        song_yml_path.display()
                    );
                    return false;
                }
            }
        } else {
            None
        };

        // Resolve section_id references to bar numbers
        let def = match resolve(&raw_def, song.as_ref()) {
            Ok(d) => d,
            Err(e) => {
                log::error!(
                    "Failed to resolve clicks definition {}: {e}",
                    def_path.display()
                );
                return false;
            }
        };

        // Interpolate to generate all click times
        let clicks = Clicks {
            clicks: interpolate(&def),
        };

        // Write clicks.yml
        let out_path = sandbox.join(&self.path);
        if let Some(parent) = out_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let yaml_out = match serde_yaml::to_string(&clicks) {
            Ok(y) => y,
            Err(e) => {
                log::error!("Failed to serialize clicks: {e}");
                return false;
            }
        };
        if let Err(e) = std::fs::write(&out_path, &yaml_out) {
            log::error!("Failed to write {}: {e}", out_path.display());
            return false;
        }

        log::info!(
            "Generated {} clicks in {}",
            clicks.clicks.len(),
            out_path.display()
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChordsSection, RawClick, SectionItem, StructureItem};
    use crate::nodes::ClickDef;

    #[test]
    fn test_interpolate_two_points() {
        // bar 1 beat 1 (abs=1) at 10s, bar 3 beat 3 (abs=11) at 20s → 10 intervals of 1s
        let def = ClicksDefinition {
            clicks: vec![
                Click {
                    bar_number: 1,
                    beat_in_bar_number: 1,
                    time: 10.0,
                    description: "bar 1".to_string(),
                },
                Click {
                    bar_number: 3,
                    beat_in_bar_number: 3,
                    time: 20.0,
                    description: "bar 3 beat 3".to_string(),
                },
            ],
        };
        let result = interpolate(&def);
        assert_eq!(result.len(), 11); // beats 1..=11
        assert!((result[0] - 10.0).abs() < 1e-9);
        assert!((result[2] - 12.0).abs() < 1e-9); // beat 3 at 12s
        assert!((result[10] - 20.0).abs() < 1e-9);
    }

    #[test]
    fn test_interpolate_three_points() {
        // bar1:beat1 (abs=1) at 0s, bar1:beat3 (abs=3) at 1s, bar2:beat1 (abs=5) at 3s
        let def = ClicksDefinition {
            clicks: vec![
                Click {
                    bar_number: 1,
                    beat_in_bar_number: 1,
                    time: 0.0,
                    description: "bar 1".to_string(),
                },
                Click {
                    bar_number: 1,
                    beat_in_bar_number: 3,
                    time: 1.0,
                    description: "bar 1 beat 3".to_string(),
                },
                Click {
                    bar_number: 2,
                    beat_in_bar_number: 1,
                    time: 3.0,
                    description: "bar 2".to_string(),
                },
            ],
        };
        let result = interpolate(&def);
        assert_eq!(result.len(), 5); // beats 1,2,3,4,5
        assert!((result[0] - 0.0).abs() < 1e-9);
        assert!((result[1] - 0.5).abs() < 1e-9);
        assert!((result[2] - 1.0).abs() < 1e-9);
        assert!((result[3] - 2.0).abs() < 1e-9);
        assert!((result[4] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_interpolate_empty() {
        let def = ClicksDefinition { clicks: vec![] };
        assert!(interpolate(&def).is_empty());
    }

    #[test]
    fn test_interpolate_single() {
        let def = ClicksDefinition {
            clicks: vec![Click {
                bar_number: 1,
                beat_in_bar_number: 1,
                time: 0.0,
                description: "start".to_string(),
            }],
        };
        let result = interpolate(&def);
        assert_eq!(result.len(), 1);
        assert!((result[0] - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_resolve_bar_number() {
        let raw = RawClicksDefinition {
            clicks: vec![RawClick {
                bar_number: Some(3),
                section_id: None,
                beat_in_bar_number: 2,
                time: 5.0,
                description: "test".to_string(),
            }],
        };
        let def = resolve(&raw, None).unwrap();
        assert_eq!(def.clicks[0].bar_number, 3);
        assert_eq!(def.clicks[0].beat_in_bar_number, 2);
    }

    #[test]
    fn test_resolve_section_id() {
        // intro: 2 bars (Am|G), so bar 1
        // couplet1: 4 bars (C|G|Am|F), starts at bar 3
        let song = Song {
            files: crate::model::SongFiles {
                lilypond: vec![],
                tex: vec![],
                mp3: vec![],
                has_clicks: false,
                has_mp3: false,
            },
            info: crate::model::SongInfo {
                title: "Test".to_string(),
                author: "Test".to_string(),
                tempo: 120,
                time_signature: None,
                tags: vec![],
                external_id: None,
            },
            meta: Default::default(),
            structure: vec![
                StructureItem {
                    id: "intro".to_string(),
                    item: SectionItem::Chords(ChordsSection {
                        title: "Intro".to_string(),
                        section_type: "intro".to_string(),
                        section_body: None,
                        color: None,
                        rows: vec!["Am|G".to_string()], // 2 bars
                        drum_sequence: None,
                    }),
                },
                StructureItem {
                    id: "couplet1".to_string(),
                    item: SectionItem::Chords(ChordsSection {
                        title: "Couplet 1".to_string(),
                        section_type: "couplet".to_string(),
                        section_body: None,
                        color: None,
                        rows: vec!["C|G|Am|F".to_string()], // 4 bars
                        drum_sequence: None,
                    }),
                },
            ],
        };

        let raw = RawClicksDefinition {
            clicks: vec![
                RawClick {
                    bar_number: None,
                    section_id: Some("intro".to_string()),
                    beat_in_bar_number: 1,
                    time: 0.0,
                    description: "intro".to_string(),
                },
                RawClick {
                    bar_number: None,
                    section_id: Some("couplet1".to_string()),
                    beat_in_bar_number: 1,
                    time: 4.0,
                    description: "couplet 1".to_string(),
                },
            ],
        };

        let def = resolve(&raw, Some(&song)).unwrap();
        assert_eq!(def.clicks[0].bar_number, 1); // intro starts at bar 1
        assert_eq!(def.clicks[1].bar_number, 3); // couplet1 starts at bar 3
    }

    #[test]
    fn test_resolve_section_id_not_found() {
        let raw = RawClicksDefinition {
            clicks: vec![RawClick {
                bar_number: None,
                section_id: Some("nonexistent".to_string()),
                beat_in_bar_number: 1,
                time: 0.0,
                description: "nope".to_string(),
            }],
        };
        let song = Song {
            files: crate::model::SongFiles {
                lilypond: vec![],
                tex: vec![],
                mp3: vec![],
                has_clicks: false,
                has_mp3: false,
            },
            info: crate::model::SongInfo {
                title: "Test".to_string(),
                author: "Test".to_string(),
                tempo: 120,
                time_signature: None,
                tags: vec![],
                external_id: None,
            },
            meta: Default::default(),
            structure: vec![],
        };
        let result = resolve(&raw, Some(&song));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonexistent"));
    }

    #[test]
    fn test_click_yml_build() {
        let sandbox = tempfile::tempdir().expect("Failed to create sandbox");
        let song_dir = sandbox.path().join("artist/song");
        std::fs::create_dir_all(&song_dir).unwrap();

        // Write a clicks-def.yml
        std::fs::write(
            song_dir.join("clicks-def.yml"),
            "clicks:\n- bar_number: 1\n  beat_in_bar_number: 1\n  time: \"0:0.0\"\n  description: bar 1\n- bar_number: 2\n  beat_in_bar_number: 1\n  time: \"0:2.0\"\n  description: bar 2\n",
        )
        .unwrap();

        // Create ClickDef as predecessor
        let def_node =
            ClickDef::new(PathBuf::from("artist/song/clicks-def.yml"), sandbox.path()).unwrap();

        let click_yml_node = ClickYml::new(PathBuf::from("artist/song/clicks.yml"));

        let predecessors: Vec<&(dyn GNode + Send + Sync)> = vec![&def_node];
        let result = click_yml_node.build(sandbox.path(), &predecessors);
        assert!(result, "build should succeed");

        // Verify the output
        let output =
            std::fs::read_to_string(sandbox.path().join("artist/song/clicks.yml")).unwrap();
        let clicks: Clicks = serde_yaml::from_str(&output).unwrap();
        assert_eq!(clicks.clicks.len(), 5);
        assert!((clicks.clicks[0] - 0.0).abs() < 1e-9);
        assert!((clicks.clicks[1] - 0.5).abs() < 1e-9);
        assert!((clicks.clicks[4] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_click_yml_build_with_section_id() {
        let sandbox = tempfile::tempdir().expect("Failed to create sandbox");
        let song_dir = sandbox.path().join("artist/song");
        std::fs::create_dir_all(&song_dir).unwrap();

        // Write song.yml: intro (2 bars), couplet1 (2 bars) → starts at bar 3
        std::fs::write(
            song_dir.join("song.yml"),
            r#"files:
  has_clicks: true
  has_mp3: true
info:
  title: Test
  author: Test
  tempo: 120
  time_signature: null
structure:
  - id: intro
    item: !Chords
      title: Intro
      type: intro
      rows:
        - 'Am|G'
  - id: couplet1
    item: !Chords
      title: Couplet 1
      type: couplet
      rows:
        - 'C|G'
"#,
        )
        .unwrap();

        // Write a clicks-def.yml using section_id
        std::fs::write(
            song_dir.join("clicks-def.yml"),
            "clicks:\n- section_id: intro\n  beat_in_bar_number: 1\n  time: \"0:0.0\"\n  description: intro\n- section_id: couplet1\n  beat_in_bar_number: 1\n  time: \"0:4.0\"\n  description: couplet 1\n",
        )
        .unwrap();

        let def_node =
            ClickDef::new(PathBuf::from("artist/song/clicks-def.yml"), sandbox.path()).unwrap();

        let click_yml_node = ClickYml::new(PathBuf::from("artist/song/clicks.yml"));

        let predecessors: Vec<&(dyn GNode + Send + Sync)> = vec![&def_node];
        let result = click_yml_node.build(sandbox.path(), &predecessors);
        assert!(result, "build with section_id should succeed");

        let output =
            std::fs::read_to_string(sandbox.path().join("artist/song/clicks.yml")).unwrap();
        let clicks: Clicks = serde_yaml::from_str(&output).unwrap();
        // intro: bar 1, beat 1 → abs beat 1; couplet1: bar 3, beat 1 → abs beat 9
        // gap = 8 beats, so 8 interpolated + 1 final = 9 total
        assert_eq!(clicks.clicks.len(), 9);
        assert!((clicks.clicks[0] - 0.0).abs() < 1e-9);
        assert!((clicks.clicks[8] - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_click_yml_missing_predecessor() {
        let sandbox = tempfile::tempdir().expect("Failed to create sandbox");
        let click_yml_node = ClickYml::new(PathBuf::from("clicks.yml"));
        let predecessors: Vec<&(dyn GNode + Send + Sync)> = vec![];
        let result = click_yml_node.build(sandbox.path(), &predecessors);
        assert!(!result, "build should fail without predecessor");
    }
}
