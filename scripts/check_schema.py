#!/usr/bin/env python3
"""
Validate components/*.toml and pin-anchors-template/*.json against
docs/component-schema.md v1.0.

Stdlib only (tomllib + json). Requires Python 3.11+.

Checks:
  - components/*.toml has required fields and valid enum values
  - pin-anchors-template/*.json is valid JSON with required fields
  - When an anchor has a matching component TOML, pins[].name and
    pins[].electrical sets must agree exactly
  - Board anchors (no matching TOML, e.g. arduino_uno.json) only get
    JSON-shape + enum validation

Exits 1 on first batch of errors. Run from repo root.
"""
from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

# 18 electrical enum values from docs/component-schema.md §四
ELECTRICAL_ENUM = {
    "digital_in", "digital_out", "analog_in", "analog_out", "pwm_in",
    "power", "gnd",
    "i2c_sda", "i2c_scl",
    "spi_mosi", "spi_miso", "spi_sck", "spi_cs",
    "uart_tx", "uart_rx",
    "one_wire", "passive", "nc",
}

CATEGORY_ENUM = {"output", "input", "display", "passive", "wiring", "power"}
DIRECTION_ENUM = {"in", "out", "bidirectional"}
SCHEMA_VERSION = "1.0"


def err(msg: str, errors: list[str]) -> None:
    errors.append(msg)


def check_component_toml(path: Path, errors: list[str]) -> dict | None:
    """Return parsed TOML if valid enough to cross-check, else None."""
    try:
        with path.open("rb") as f:
            data = tomllib.load(f)
    except tomllib.TOMLDecodeError as e:
        err(f"{path}: invalid TOML — {e}", errors)
        return None

    ct = data.get("component_type")
    if not isinstance(ct, dict):
        err(f"{path}: missing [component_type] table", errors)
        return None

    for field in ("id", "display_name", "category", "schema_version"):
        if field not in ct:
            err(f"{path}: [component_type].{field} missing", errors)

    cid = ct.get("id")
    if cid and path.stem != cid:
        err(f"{path}: id `{cid}` does not match filename `{path.stem}`", errors)

    cat = ct.get("category")
    if cat and cat not in CATEGORY_ENUM:
        err(
            f"{path}: category `{cat}` not in {sorted(CATEGORY_ENUM)}",
            errors,
        )

    ver = ct.get("schema_version")
    if ver and ver != SCHEMA_VERSION:
        err(
            f"{path}: schema_version `{ver}` != `{SCHEMA_VERSION}`",
            errors,
        )

    pins = data.get("pin", [])
    if not pins:
        err(f"{path}: at least one [[pin]] required", errors)

    seen_names: set[str] = set()
    for i, pin in enumerate(pins):
        loc = f"{path}: [[pin]] #{i}"
        for field in ("name", "electrical", "direction"):
            if field not in pin:
                err(f"{loc}: missing `{field}`", errors)

        name = pin.get("name")
        if name in seen_names:
            err(f"{loc}: duplicate pin name `{name}`", errors)
        if name:
            seen_names.add(name)

        elec = pin.get("electrical")
        if elec and elec not in ELECTRICAL_ENUM:
            err(
                f"{loc}: electrical `{elec}` not in 18-enum",
                errors,
            )

        direction = pin.get("direction")
        if direction and direction not in DIRECTION_ENUM:
            err(
                f"{loc}: direction `{direction}` not in {sorted(DIRECTION_ENUM)}",
                errors,
            )

    return data


def check_anchor_json(
    path: Path, components: dict[str, dict], errors: list[str]
) -> None:
    try:
        with path.open("rb") as f:
            data = json.load(f)
    except json.JSONDecodeError as e:
        err(f"{path}: invalid JSON — {e}", errors)
        return

    for field in ("format_version", "component_id", "pins"):
        if field not in data:
            err(f"{path}: missing `{field}`", errors)

    if data.get("format_version") not in (None, SCHEMA_VERSION):
        err(
            f"{path}: format_version `{data['format_version']}` != `{SCHEMA_VERSION}`",
            errors,
        )

    cid = data.get("component_id")
    pins = data.get("pins", [])
    if not isinstance(pins, list) or not pins:
        err(f"{path}: pins[] must be non-empty array", errors)
        return

    anchor_pin_map: dict[str, str] = {}
    for i, pin in enumerate(pins):
        loc = f"{path}: pins[{i}]"
        name = pin.get("name")
        elec = pin.get("electrical")
        # Documentation placeholders carry only `_`-prefixed keys
        # (e.g. breadboard.json's batch-gen TODO marker). Skip them.
        if not name and not elec and all(k.startswith("_") for k in pin):
            continue
        if not name:
            err(f"{loc}: missing `name`", errors)
            continue
        if name in anchor_pin_map:
            err(f"{loc}: duplicate pin name `{name}`", errors)
        if not elec:
            err(f"{loc}: missing `electrical`", errors)
        elif elec not in ELECTRICAL_ENUM:
            err(f"{loc}: electrical `{elec}` not in 18-enum", errors)
        anchor_pin_map[name] = elec or ""

    # Cross-check against components/<cid>.toml if it exists.
    comp = components.get(cid)
    if comp is None:
        # Board-level anchor (e.g. arduino_uno) or orphan; skip pair check.
        return

    toml_pin_map = {p["name"]: p.get("electrical", "") for p in comp.get("pin", [])}

    missing = set(toml_pin_map) - set(anchor_pin_map)
    extra = set(anchor_pin_map) - set(toml_pin_map)
    if missing:
        err(
            f"{path}: missing pins from components/{cid}.toml: {sorted(missing)}",
            errors,
        )
    if extra:
        err(
            f"{path}: extra pins not in components/{cid}.toml: {sorted(extra)}",
            errors,
        )

    for name in set(anchor_pin_map) & set(toml_pin_map):
        a_elec = anchor_pin_map[name]
        t_elec = toml_pin_map[name]
        if a_elec != t_elec:
            err(
                f"{path}: pin `{name}` electrical mismatch — "
                f"anchor `{a_elec}` vs toml `{t_elec}`",
                errors,
            )


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    comp_dir = root / "components"
    anchor_dir = root / "pin-anchors-template"

    if not comp_dir.is_dir():
        print(f"error: {comp_dir} does not exist", file=sys.stderr)
        return 1
    if not anchor_dir.is_dir():
        print(f"error: {anchor_dir} does not exist", file=sys.stderr)
        return 1

    errors: list[str] = []
    components: dict[str, dict] = {}

    toml_files = sorted(comp_dir.glob("*.toml"))
    for p in toml_files:
        data = check_component_toml(p, errors)
        if data is not None:
            cid = data["component_type"].get("id") or p.stem
            components[cid] = data

    json_files = sorted(anchor_dir.glob("*.json"))
    for p in json_files:
        check_anchor_json(p, components, errors)

    if errors:
        print(f"check_schema: {len(errors)} error(s)", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1

    print(
        f"check_schema: OK — {len(toml_files)} component(s), "
        f"{len(json_files)} anchor(s)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
