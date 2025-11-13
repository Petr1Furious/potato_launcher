from abc import abstractmethod
from typing import Protocol

from app.models.setting import Setting


class SettingGateway(Protocol):

    @abstractmethod
    def get_all(self) -> list[Setting]:
        raise NotImplementedError

    @abstractmethod
    def update(self, settings: list[Setting]) -> list[Setting]:
        raise NotImplementedError
