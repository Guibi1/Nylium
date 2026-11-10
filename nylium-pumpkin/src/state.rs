use std::collections::{BTreeMap, HashSet};

use nylium_adapter::{Field, FieldKind, FieldValue, Player};
use pumpkin_core::{server::Server, world::World};
use pumpkin_data::game_rules::{GameRule, GameRuleValue};

/// Returns online players and offline operators; Pumpkin exposes no iterator for other cached profiles.
pub(crate) fn players(server: &Server) -> Vec<Player> {
    let mut players = BTreeMap::new();
    let operators = server
        .data
        .operator_config
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let offline_operators: Vec<_> = operators
        .ops
        .iter()
        .map(|entry| (entry.uuid, entry.name.clone()))
        .collect();
    let operator_ids: HashSet<_> = offline_operators.iter().map(|(id, _)| *id).collect();
    drop(operators);
    for world in server.worlds.load().iter() {
        for player in world.players.load().iter() {
            let id = player.gameprofile.id;
            players.insert(
                id,
                Player {
                    id,
                    name: player.gameprofile.name.clone(),
                    online: true,
                    dimension: world.dimension.minecraft_name.to_owned(),
                    operator: operator_ids.contains(&id),
                },
            );
        }
    }
    let mut user_cache = server
        .data
        .user_cache
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (id, name) in offline_operators {
        players.entry(id).or_insert_with(|| Player {
            id,
            name: user_cache
                .get_by_uuid(id)
                .map_or(name, |profile| profile.name),
            online: false,
            dimension: String::new(),
            operator: true,
        });
    }
    players.into_values().collect()
}

pub(crate) fn rules(server: &Server) -> Vec<Field> {
    let mut fields = Vec::new();
    for world in server.worlds.load().iter() {
        for rule in GameRule::all() {
            let value = world.get_game_rule(rule);
            fields.push(rule_field(world.dimension.minecraft_name, rule, value));
        }
    }
    fields.sort_by(|a, b| a.id.cmp(&b.id));
    fields
}

fn rule_field(world: &str, rule: &GameRule, current: GameRuleValue<i64, bool>) -> Field {
    let (value, kind) = match current {
        GameRuleValue::Bool(value) => (FieldValue::Boolean(value), FieldKind::Boolean),
        GameRuleValue::Int(value) => (FieldValue::Text(value.to_string()), FieldKind::Text),
    };
    Field {
        id: format!("{world}#minecraft:{rule}"),
        label: format!("{world} / {}", rule.to_string().replace('_', " ")),
        description: "Minecraft game rule".into(),
        value,
        kind,
    }
}

pub(crate) fn set_rule(server: &Server, id: &str, value: FieldValue) -> Result<(), String> {
    let (world_name, rule_name) = id
        .split_once('#')
        .ok_or_else(|| format!("unknown game rule '{id}'"))?;
    let rule_name = rule_name.strip_prefix("minecraft:").unwrap_or(rule_name);
    let worlds = server.worlds.load();
    let world = worlds
        .iter()
        .find(|world| world.dimension.minecraft_name == world_name)
        .cloned()
        .ok_or_else(|| format!("unknown world in game rule '{id}'"))?;
    let rule = GameRule::all()
        .iter()
        .find(|rule| rule.to_string() == rule_name)
        .ok_or_else(|| format!("unknown game rule '{id}'"))?;
    let next = parse_rule_value(world.get_game_rule(rule), value)?;
    persist_rule(&world, rule, next)
}

fn parse_rule_value(
    current: GameRuleValue<i64, bool>,
    value: FieldValue,
) -> Result<GameRuleValue<i64, bool>, String> {
    match (current, value) {
        (GameRuleValue::Bool(_), FieldValue::Boolean(value)) => Ok(GameRuleValue::Bool(value)),
        (GameRuleValue::Int(_), FieldValue::Number(value)) => {
            Ok(GameRuleValue::Int(i64::from(value)))
        }
        (GameRuleValue::Int(_), FieldValue::Text(value)) => Ok(GameRuleValue::Int(
            value.parse().map_err(|_| "Enter a signed 64-bit integer")?,
        )),
        _ => Err("Game rule value has the wrong type".into()),
    }
}

fn persist_rule(
    world: &World,
    rule: &GameRule,
    value: GameRuleValue<i64, bool>,
) -> Result<(), String> {
    let current = world.level_info.load();
    let mut registry = current.game_rules.clone();
    match (registry.get_mut(rule), &value) {
        (GameRuleValue::Bool(target), GameRuleValue::Bool(value)) => *target = *value,
        (GameRuleValue::Int(target), GameRuleValue::Int(value)) => *target = *value,
        _ => return Err("Game rule value has the wrong type".into()),
    }
    pumpkin_world::world_info::data_files::write_game_rules(
        &world.level.level_folder.root_folder,
        &registry,
        current.data_version,
    )
    .map_err(|error| format!("Failed to persist game rule: {error}"))?;
    world.set_game_rule(rule, value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_game_rules_keep_signed_values_and_reject_bad_input() {
        let field = rule_field(
            "minecraft:overworld",
            &GameRule::PlayersSleepingPercentage,
            GameRuleValue::Int(-2),
        );
        assert_eq!(field.value, FieldValue::Text("-2".into()));
        assert_eq!(
            rule_field(
                "minecraft:overworld",
                &GameRule::PlayersSleepingPercentage,
                GameRuleValue::Int(100)
            )
            .kind,
            FieldKind::Text
        );
        assert!(matches!(
            parse_rule_value(GameRuleValue::Int(0), FieldValue::Text("-2".into())),
            Ok(GameRuleValue::Int(-2))
        ));
        assert!(parse_rule_value(GameRuleValue::Int(0), FieldValue::Text("nope".into())).is_err());
        assert!(parse_rule_value(GameRuleValue::Bool(false), FieldValue::Number(1)).is_err());
    }
}
