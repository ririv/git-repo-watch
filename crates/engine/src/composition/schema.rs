//! Native output contract shared by all composition agents.
use super::Unit;
use serde_json::{Value, json};

pub(super) fn plan(units: &[Unit]) -> Value {
    let ids = units.iter().map(|unit| &unit.id).collect::<Vec<_>>();
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["commits"],
        "properties": {
            "commits": {
                "type": "array",
                "minItems": 1,
                "maxItems": 64,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["message", "units"],
                    "properties": {
                        "message": {"type": "string"},
                        "units": {
                            "type": "array",
                            "minItems": 1,
                            "maxItems": units.len(),
                            "items": {"type": "string", "enum": ids}
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cancel::Cancel, composition::capture};
    #[test]
    fn contract_uses_the_captured_unit_ids_across_both_layers() {
        let f = test_support::Fixture::create();
        test_support::write(f.repo.join("staged.txt"), "staged\n");
        test_support::git(&f.repo, &["add", "staged.txt"]);
        test_support::write(f.repo.join("new.txt"), "new\n");
        let snapshot = capture(&f.repo, &Cancel::default()).unwrap();
        let schema = plan(&snapshot.units);
        let step = &schema["properties"]["commits"]["items"];
        assert_eq!(
            step["properties"]["units"]["items"]["enum"],
            json!(["S1", "W2"])
        );
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(step["additionalProperties"], false);
        assert_eq!(step["required"], json!(["message", "units"]));
    }
}
