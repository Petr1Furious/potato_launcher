from dataclasses import asdict
from typing import Any

from app.gateway.json.db import read_file, save_file
from app.gateway.settings import SettingGateway
from app.models.setting import Setting


class JsonSettingGateway(SettingGateway):

    def get_all(self) -> list[Setting]:
        data = read_file()
        settings = []
        for raw_setting in data.get("settings", []):
            settings.append(
                Setting(
                    key=raw_setting["key"],
                    value=raw_setting["value"],
                    type=raw_setting["type"],
                )
            )
        return settings

    def _setting_to_dict(self, setting: Setting) -> dict[str, Any]:
        data = asdict(setting)
        data["type"] = data["type"].value
        return data

    def update(self, settings: list[Setting]) -> list[Setting]:
        data = read_file()
        existing_settings = data.get("settings", [])
        index_by_key = {item.get("key"): i for i, item in enumerate(existing_settings)}

        for setting in settings:
            item_json = self._setting_to_dict(setting)
            key = item_json["key"]
            if key in index_by_key:
                if existing_settings[index_by_key[key]]["type"] != setting.type.value:
                    raise ValueError(
                        f"Incorrect string type {setting.type.value}. Expected {existing_settings[index_by_key[key]]['type']}"
                    )
                existing_settings[index_by_key[key]] = item_json
            else:
                index_by_key[key] = len(existing_settings)
                existing_settings.append(item_json)

        data["settings"] = existing_settings
        save_file(data)

        return [Setting(**s) for s in existing_settings]
