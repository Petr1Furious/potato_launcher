import json
import os.path
from typing import Any

from app.config import config


_DB_FILE_NAME = "db.json"


def read_file() -> dict[str, Any]:
    if os.path.exists(_DB_FILE_NAME):
        with open(_DB_FILE_NAME, "r") as db_file:
            return json.load(db_file)
    else:
        with open(_DB_FILE_NAME, "w") as db_file:
            json.dump({}, db_file)
        return {}


def save_file(data: dict[str, Any]) -> None:
    with open(_DB_FILE_NAME, "w") as db_file:
        json.dump(data, db_file)


def save_spec_file(data: dict[str, Any]) -> None:
    with open(config.SPEC_FILE, "w") as spec_file:
        json.dump(data, spec_file)
        print(f"saved to {config.SPEC_FILE}")
