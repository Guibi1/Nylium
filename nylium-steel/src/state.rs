use std::{collections::BTreeMap, sync::Arc};
use steel_core::player::connection::NetworkConnection;

use nylium_adapter::{Field, FieldKind, FieldValue, Player};
use steel_core::{server::Server, world::World};
use steel_registry::{
    REGISTRY,
    game_rules::{ErasedGameRuleRef, GameRuleType, GameRuleValue},
};

pub(crate) fn players(server: &Server) -> Vec<Player> {
    let mut players = BTreeMap::new();
    for known in server.known_players().entries() {
        let uuid = known.uuid();
        players.insert(
            uuid,
            Player {
                id: uuid,
                name: known.last_known_name().to_owned(),
                online: false,
                dimension: String::new(),
                // Steel keeps OP_GROUP private; this value is pinned to the inspected API revision.
                operator: server
                    .player_permission_state(uuid)
                    .is_some_and(|state| state.groups().iter().any(|group| group == "op")),
            },
        );
    }
    for world in server.worlds.values() {
        world.players.iter_players(|uuid, player| {
            if !player.connection.closed() {
                players.insert(
                    *uuid,
                    Player {
                        id: *uuid,
                        name: player.gameprofile.name().to_owned(),
                        online: true,
                        dimension: world.key.to_string(),
                        operator: server
                            .player_permission_state(*uuid)
                            .is_some_and(|state| state.groups().iter().any(|group| group == "op")),
                    },
                );
            }
            true
        });
    }
    players.into_values().collect()
}

pub(crate) fn game_rules(server: &Server) -> Vec<Field> {
    let mut fields = Vec::new();
    for world in server.worlds.values() {
        for (_, rule) in REGISTRY.game_rules.iter() {
            if let Some(field) = rule_field(
                &world.key.to_string(),
                rule,
                world.get_erased_game_rule(rule),
            ) {
                fields.push(field);
            }
        }
    }
    fields.sort_by(|a, b| a.id.cmp(&b.id));
    fields
}

fn rule_field(world: &str, rule: ErasedGameRuleRef, current: GameRuleValue) -> Option<Field> {
    let (value, kind) = match rule.value_type() {
        GameRuleType::Bool => (
            FieldValue::Boolean(*current.downcast_ref::<bool>()?),
            FieldKind::Boolean,
        ),
        GameRuleType::Int => {
            let value = *current.downcast_ref::<i32>()?;
            if rule.min_value().unwrap_or(i32::MIN) >= 0 && value >= 0 {
                (
                    FieldValue::Number(u32::try_from(value).ok()?),
                    FieldKind::Number {
                        min: rule.min_value().and_then(|min| u32::try_from(min).ok()),
                        max: Some(rule.max_value().unwrap_or(i32::MAX) as u32),
                    },
                )
            } else {
                (FieldValue::Text(value.to_string()), FieldKind::Text)
            }
        }
    };
    Some(Field {
        id: format!("{}#{}", world, rule.key()),
        label: format!(
            "{} / {}",
            world,
            rule.key().to_string().replace('_', " ").replace(':', " / ")
        ),
        description: format!(
            "{:?} game rule{}",
            rule.category(),
            match (rule.min_value(), rule.max_value()) {
                (Some(min), Some(max)) => format!(" (range {min}–{max})"),
                (Some(min), None) => format!(" (minimum {min})"),
                (None, Some(max)) => format!(" (maximum {max})"),
                (None, None) => String::new(),
            }
        ),
        value,
        kind,
    })
}

pub(crate) fn set_rule(server: &Server, id: &str, value: FieldValue) -> Result<Arc<World>, String> {
    let (world_key, rule_key) = id
        .split_once('#')
        .ok_or_else(|| format!("unknown game rule '{id}'"))?;
    let world = server
        .worlds
        .values()
        .find(|world| world.key.to_string() == world_key)
        .cloned()
        .ok_or_else(|| format!("unknown world in game rule '{id}'"))?;
    let rule = REGISTRY
        .game_rules
        .iter()
        .map(|(_, rule)| rule)
        .find(|rule| rule.key().to_string() == rule_key)
        .ok_or_else(|| format!("unknown game rule '{id}'"))?;
    let value = parse_rule_value(rule, value)?;
    if !world.set_erased_game_rule(rule, value) {
        return Err(format!("Cannot set game rule '{id}'"));
    }
    Ok(world)
}

fn parse_rule_value(rule: ErasedGameRuleRef, value: FieldValue) -> Result<GameRuleValue, String> {
    let value = match (rule.value_type(), value) {
        (GameRuleType::Bool, FieldValue::Boolean(value)) => GameRuleValue::new(value),
        (GameRuleType::Int, FieldValue::Number(value))
            if rule.min_value().unwrap_or(i32::MIN) >= 0 =>
        {
            GameRuleValue::new(
                i32::try_from(value).map_err(|_| "Game rule is outside the i32 range")?,
            )
        }
        (GameRuleType::Int, FieldValue::Text(value))
            if rule.min_value().unwrap_or(i32::MIN) < 0 =>
        {
            GameRuleValue::new(
                value
                    .parse::<i32>()
                    .map_err(|_| "Enter a signed 32-bit integer")?,
            )
        }
        _ => return Err("Game rule value has the wrong type".into()),
    };
    if !rule.validates_erased_value(&value) {
        return Err(format!("Value for {} is outside its bounds", rule.key()));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use steel_registry::game_rules::{GameRule, GameRuleCategory};
    use steel_utils::Identifier;

    static SIGNED: GameRule<i32> = GameRule::integer(
        Identifier::vanilla_static("signed_test"),
        GameRuleCategory::Misc,
        0,
        Some(-10),
        Some(10),
    );
    static UNSIGNED: GameRule<i32> = GameRule::integer(
        Identifier::vanilla_static("unsigned_test"),
        GameRuleCategory::Misc,
        0,
        Some(0),
        None,
    );
    static BOOL: GameRule<bool> = GameRule::boolean(
        Identifier::vanilla_static("bool_test"),
        GameRuleCategory::Misc,
        false,
    );

    #[test]
    fn converts_native_rules_without_losing_signed_values_or_integer_bounds() {
        let field = rule_field("minecraft:overworld", &SIGNED, GameRuleValue::new(-3_i32)).unwrap();
        assert_eq!(field.id, "minecraft:overworld#minecraft:signed_test");
        assert_eq!(field.value, FieldValue::Text("-3".into()));
        assert_eq!(field.kind, FieldKind::Text);
        let parsed = parse_rule_value(&SIGNED, FieldValue::Text("-4".into())).unwrap();
        assert_eq!(parsed.downcast_ref::<i32>(), Some(&-4));
        assert!(parse_rule_value(&SIGNED, FieldValue::Text("-11".into())).is_err());
        assert!(parse_rule_value(&SIGNED, FieldValue::Text("2147483648".into())).is_err());
        assert!(parse_rule_value(&BOOL, FieldValue::Number(1)).is_err());
        let field =
            rule_field("minecraft:overworld", &UNSIGNED, GameRuleValue::new(1_i32)).unwrap();
        assert_eq!(
            field.kind,
            FieldKind::Number {
                min: Some(0),
                max: Some(i32::MAX as u32)
            }
        );
        assert!(parse_rule_value(&UNSIGNED, FieldValue::Number(u32::MAX)).is_err());
    }
}
