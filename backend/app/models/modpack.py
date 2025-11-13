from dataclasses import dataclass, asdict
from typing import cast, Literal, Annotated, Union
from pydantic import BaseModel, Field, TypeAdapter
from enum import Enum

from app.models.base import ApiModel


class LoaderType(str, Enum):
    VANILLA = "vanilla"
    FORGE = "forge"
    FABRIC = "fabric"
    NEOFORGE = "neoforge"


class ProcessPath(BaseModel):
    path: str
    override: bool
    is_recursive: bool


@dataclass
class AuthType:
    kind: Literal["mojang", "telegram", "ely.by", "offline"]


@dataclass
class MojangAuth:
    kind: Literal["mojang"] = "mojang"


@dataclass
class TelegramAuth:
    auth_base_url: str
    kind: Literal["telegram"] = "telegram"


@dataclass
class ElyAuth:
    client_id: str
    client_secret: str
    kind: Literal["ely.by"] = "ely.by"


@dataclass
class OfflineAuth:
    kind: Literal["offline"] = "offline"


AuthConfig = MojangAuth | TelegramAuth | ElyAuth | OfflineAuth


@dataclass
class Modpack:
    id: int
    name: str
    minecraft_version: str
    loader: LoaderType
    loader_version: str
    auth_config: AuthConfig


class MojangAuth(ApiModel):
    kind: Literal["mojang"] = "mojang"


class TelegramAuth(ApiModel):
    kind: Literal["telegram"] = "telegram"
    auth_base_url: str


class ElyAuth(ApiModel):
    kind: Literal["ely.by"] = "ely.by"
    client_id: str
    client_secret: str


class OfflineAuth(ApiModel):
    kind: Literal["offline"] = "offline"


AuthConfigPyd = Annotated[
    Union[MojangAuth, TelegramAuth, ElyAuth, OfflineAuth], Field(discriminator="kind")
]
AuthAdapter = TypeAdapter(AuthConfigPyd)


def to_dc(model: AuthConfigPyd) -> AuthConfig:
    clazz = AuthAdapter.dump_python(model)
    kind = clazz["kind"]
    if kind == "mojang":
        return MojangAuth()
    elif kind == "telegram":
        return TelegramAuth(**clazz)
    elif kind == "ely.by":
        return ElyAuth(**clazz)
    elif kind == "offline":
        return OfflineAuth()
    else:
        raise ValueError(f"Unknown auth kind: {kind}")


def to_model(clazz: AuthConfig) -> AuthConfigPyd:
    d = asdict(clazz)
    return AuthAdapter.validate_python(d)


class ModpackBase(ApiModel):
    name: str
    minecraft_version: str
    loader: LoaderType
    loader_version: str
    auth_config: AuthConfigPyd

    def to_model(self) -> Modpack:
        return Modpack(
            id=cast(int, None),
            name=self.name,
            minecraft_version=self.minecraft_version,
            loader=self.loader,
            loader_version=self.loader_version,
            auth_config=to_dc(self.auth_config),
        )


class CreateModpackRequest(ModpackBase): ...


class UpdateModpackRequest(ModpackBase):

    def as_model(self, id: int) -> Modpack:
        return Modpack(
            id=id,
            name=self.name,
            minecraft_version=self.minecraft_version,
            loader=self.loader,
            loader_version=self.loader_version,
            auth_config=to_dc(self.auth_config),
        )


class ModpackResponse(ModpackBase):
    id: int

    def to_model(self) -> Modpack:
        return Modpack(
            id=self.id,
            name=self.name,
            minecraft_version=self.minecraft_version,
            loader=self.loader,
            loader_version=self.loader_version,
            auth_config=to_dc(self.auth_config),
        )
