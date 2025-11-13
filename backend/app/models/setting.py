from dataclasses import dataclass

from enum import Enum

from app.models.base import ApiModel


class SettingType(str, Enum):
    STRING = "string"
    BOOLEAN = "boolean"


@dataclass
class Setting:
    key: str
    type: SettingType
    value: str | bool


class SettingBase(ApiModel):
    key: str
    value: str | bool
    type: SettingType

    def to_model(self) -> Setting:
        return Setting(key=self.key, type=self.type, value=self.value)


class SettingRequest(SettingBase): ...


class UpdateSettingRequest(SettingBase): ...


class SettingResponse(SettingBase): ...
