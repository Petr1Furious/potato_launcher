from abc import abstractmethod
from typing import Protocol

from app.models.modpack import Modpack


class ModpackGateway(Protocol):

    @abstractmethod
    def get_all(self) -> list[Modpack]:
        raise NotImplementedError

    @abstractmethod
    def get_by_id(self, id: int) -> Modpack | None:
        raise NotImplementedError

    @abstractmethod
    def update(self, modpack: Modpack) -> Modpack:
        raise NotImplementedError

    @abstractmethod
    def delete(self, id: int) -> None:
        raise NotImplementedError

    @abstractmethod
    def save(self, modpack: Modpack) -> Modpack:
        raise NotImplementedError

    @abstractmethod
    def generate_spec(self) -> None:
        raise NotImplementedError
