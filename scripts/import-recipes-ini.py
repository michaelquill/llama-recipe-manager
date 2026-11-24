#!/usr/bin/env python3
"""Import recipes from llama-server presets INI format into Llama Recipe Manager SQLite database."""

import argparse
import os
import re
import sqlite3
import sys
import uuid
from datetime import datetime, timezone
from pathlib import Path

DEFAULT_DB_PATH = Path.home() / ".local/share/com.ashar.llama-recipe-manager/recipes.db"

# Flags managed directly by the app / Settings; forbidden in recipe.command
FORBIDDEN_FLAGS = {
    "port", "p", "host",
    "m", "model", "model-path", "mmproj", "mm", "mmproj-url", "mmu",
    "log-file", "api-key", "api-key-file", "ssl-cert-file", "ssl-key-file",
    "hf-token", "hft", "api-prefix", "timeout", "to", "log-verbosity", "lv",
    "webui", "no-webui", "metrics", "slots", "no-slots",
}

SHORT_FLAGS = {"c", "t", "b", "n", "ngl", "np"}


def parse_ini(content: str, remaps: dict[str, str] = None) -> tuple[list[dict], dict[str, str]]:
    if remaps is None:
        remaps = {}

    lines = content.splitlines()

    # Pass 1: Scan for all sections and discover all variables
    sections: list[dict] = []
    current_section = None
    accumulated_comments: list[str] = []

    for line in lines:
        line_str = line.strip()
        if not line_str:
            continue
        if line_str.startswith(";") or line_str.startswith("#"):
            accumulated_comments.append(line_str.lstrip(";# ").strip())
            continue

        if line_str.startswith("[") and line_str.endswith("]"):
            sec_name = line_str[1:-1].strip()
            current_section = {
                "name": sec_name,
                "comments": "\n".join(accumulated_comments),
                "pairs": [],
            }
            sections.append(current_section)
            accumulated_comments = []
            continue

        if "=" in line_str and current_section is not None:
            k, v = line_str.split("=", 1)
            current_section["pairs"].append((k.strip(), v.strip()))

    # Find global [*] section
    global_pairs: dict[str, str] = {}
    root_vars: dict[str, str] = {}

    for sec in sections:
        if sec["name"] == "*":
            for k, v in sec["pairs"]:
                global_pairs[k] = v
                if "-root" in k or "-dir" in k or k == "llama-root":
                    root_vars[k] = v
        else:
            for k, v in sec["pairs"]:
                if "-root" in k or "-dir" in k or k == "llama-root":
                    root_vars[k] = v

    # Find any placeholder variable referenced in {var}
    var_pattern = re.compile(r"\{([a-zA-Z0-9_-]+)\}")
    for sec in sections:
        for _, v in sec["pairs"]:
            for match in var_pattern.findall(v):
                if match not in root_vars:
                    root_vars[match] = ""

    # Pass 2: Build recipes for each non-[*] section
    recipes: list[dict] = []

    for sec in sections:
        name = sec["name"]
        if name == "*":
            continue

        # Merge [*] defaults with section pairs
        sec_dict: dict[str, str] = {}
        # Apply global pairs first
        for k, v in global_pairs.items():
            sec_dict[k] = v
        # Override with section pairs
        for k, v in sec["pairs"]:
            sec_dict[k] = v

        # Build symbol table for this section
        sym_table = dict(root_vars)
        # Apply any section-level root overrides
        for k, v in sec_dict.items():
            if k in root_vars or "-root" in k or "-dir" in k:
                sym_table[k] = v
        # Apply user remaps (highest precedence)
        for k, v in remaps.items():
            if v:
                sym_table[k] = v

        def expand_vars(val: str) -> str:
            def repl(m):
                var_name = m.group(1)
                return sym_table.get(var_name, m.group(0))
            return var_pattern.sub(repl, val)

        # Model is required
        raw_model = sec_dict.get("model")
        if not raw_model:
            continue
        model_path = expand_vars(raw_model)

        raw_mmproj = sec_dict.get("mmproj", "")
        mmproj_path = expand_vars(raw_mmproj) if raw_mmproj else ""

        # Extract comments / metadata
        desc = sec["comments"]
        tags_list = []
        gpu_info = ""

        if "runner" in sec_dict:
            tags_list.append(f"runner:{sec_dict['runner']}")

        # Build command flags
        cmd_parts: list[str] = []

        for k, raw_v in sec_dict.items():
            # Skip keys that are handled separately or forbidden
            if k in ("model", "mmproj", "alias"):
                continue
            if k in root_vars or k.endswith("-root") or k.endswith("-dir"):
                continue
            if k in FORBIDDEN_FLAGS:
                continue
            if k == "runner":
                continue

            v = expand_vars(raw_v)

            # Speculative draft model flag conversion
            if k == "spec-draft-model":
                cmd_parts.extend(["--model-draft", v])
                continue

            # Boolean true / on
            if v.lower() in ("true", "on"):
                if k in ("flash-attn", "fit", "cnv", "reasoning-preserve"):
                    cmd_parts.extend([f"--{k}", v.lower()])
                else:
                    cmd_parts.append(f"--{k}")
            elif v.lower() in ("false", "off"):
                # If false / off, only emit if meaningful (e.g. mlock=false is default, so omit)
                if k == "mlock":
                    pass
                else:
                    cmd_parts.append(f"--no-{k}")
            else:
                # Key with value
                if k in SHORT_FLAGS:
                    cmd_parts.extend([f"-{k}", v])
                else:
                    cmd_parts.extend([f"--{k}", v])

        command = " ".join(cmd_parts)

        recipes.append({
            "name": name,
            "description": desc,
            "model_path": model_path,
            "mmproj_path": mmproj_path,
            "command": command,
            "gpu_info": gpu_info,
            "tags": ", ".join(tags_list),
        })

    return recipes, root_vars


def get_unused_name(conn: sqlite3.Connection, base: str) -> str:
    candidate = base
    n = 1
    while True:
        row = conn.execute("SELECT 1 FROM recipes WHERE name = ?", (candidate,)).fetchone()
        if not row:
            return candidate
        n += 1
        candidate = f"{base} (imported)" if n == 2 else f"{base} (imported {n - 1})"


def import_into_db(recipes: list[dict], db_path: Path) -> int:
    conn = sqlite3.connect(db_path)
    now = datetime.now(timezone.utc).isoformat()
    imported_count = 0

    with conn:
        for r in recipes:
            final_name = get_unused_name(conn, r["name"])
            rec_id = str(uuid.uuid4())
            conn.execute(
                """
                INSERT INTO recipes (id, name, description, command, model_path, mmproj_path, gpu_info, tags, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    rec_id,
                    final_name,
                    r["description"],
                    r["command"],
                    r["model_path"],
                    r["mmproj_path"],
                    r["gpu_info"],
                    r["tags"],
                    now,
                    now,
                ),
            )
            imported_count += 1

    conn.close()
    return imported_count


def main():
    parser = argparse.ArgumentParser(description="Import llama.cpp preset INI into Llama Recipe Manager.")
    parser.add_argument("ini_path", type=Path, help="Path to input .ini file")
    parser.add_argument("db_path", type=Path, nargs="?", default=DEFAULT_DB_PATH, help="Path to SQLite database")
    parser.add_argument("--remap", action="append", help="Remap root variable, e.g. --remap llama-root=/path/to/models")
    parser.add_argument("--dry-run", action="store_true", help="Parse and preview without inserting into DB")
    args = parser.parse_args()

    remaps = {}
    if args.remap:
        for item in args.remap:
            if "=" in item:
                k, v = item.split("=", 1)
                remaps[k.strip()] = v.strip()

    if not args.ini_path.is_file():
        sys.exit(f"Error: file not found: {args.ini_path}")

    content = args.ini_path.read_text(encoding="utf-8")
    recipes, root_vars = parse_ini(content, remaps)

    print(f"Found {len(recipes)} recipes in {args.ini_path.name}")
    print(f"Discovered root variables: {root_vars}")
    if remaps:
        print(f"Active remaps: {remaps}")

    if args.dry_run:
        print("\n--- Dry Run: First 3 Recipes Preview ---")
        for r in recipes[:3]:
            print(f"[{r['name']}]")
            print(f"  model_path : {r['model_path']}")
            print(f"  mmproj_path: {r['mmproj_path']}")
            print(f"  command    : {r['command']}")
            if r['description']:
                print(f"  description: {r['description'][:60]}...")
            print()
        print(f"Total {len(recipes)} recipes ready to import.")
        return

    count = import_into_db(recipes, args.db_path)
    print(f"Successfully imported {count} recipes into {args.db_path}")


if __name__ == "__main__":
    main()
