from typing import Any

from app.config import config
from app.gateway.json.db import read_file, save_file, save_spec_file
from app.gateway.modpack import ModpackGateway
from app.models.modpack import (
    Modpack,
    TelegramAuth,
    ElyAuth,
    MojangAuth,
    OfflineAuth,
    AuthConfig,
)


class JsonModpackGateway(ModpackGateway):
    def _auth_as_dict(self, modpack: Modpack) -> dict[str, Any]:
        auth = {
            "kind": modpack.auth_config.kind,
        }
        if modpack.auth_config.kind == "ely.by":
            auth["client_id"] = modpack.auth_config.client_id
            auth["client_secret"] = modpack.auth_config.client_secret
        elif modpack.auth_config.kind == "telegram":
            auth["auth_base_url"] = modpack.auth_config.auth_base_url
        return auth

    def _dict_to_auth(self, data: dict[str, Any]) -> AuthConfig:
        kind = data["kind"]
        if kind == "telegram":
            return TelegramAuth(auth_base_url=data["auth_base_url"])
        elif kind == "ely.by":
            return ElyAuth(
                client_id=data["client_id"], client_secret=data["client_secret"]
            )
        elif kind == "mojang":
            return MojangAuth()
        elif kind == "offline":
            return OfflineAuth()
        else:
            raise ValueError(f"Unknown auth kind: {kind}")

    def save(self, modpack: Modpack) -> Modpack:
        data = read_file()
        modpacks = data.get("modpacks", [])

        new_id = max((int(m.get("id", 0)) for m in modpacks), default=0) + 1

        modpacks.append(
            {
                "id": new_id,
                "name": modpack.name,
                "minecraft_version": modpack.minecraft_version,
                "loader": modpack.loader,
                "loader_version": modpack.loader_version,
                "auth_config": self._auth_as_dict(modpack),
            }
        )
        data["modpacks"] = modpacks
        save_file(data)

        return Modpack(
            id=new_id,
            name=modpack.name,
            minecraft_version=modpack.minecraft_version,
            loader=modpack.loader.value,
            loader_version=modpack.loader_version,
            auth_config=self._dict_to_auth(modpacks[-1]["auth_config"]),
        )

    def get_by_id(self, id: int) -> Modpack | None:
        data = read_file()
        modpacks = data.get("modpacks", [])

        for raw_modpack in modpacks:
            if raw_modpack["id"] == id:
                return Modpack(**raw_modpack)
        return None

    def generate_spec(self) -> None:
        data = read_file()
        modpacks = data.get("modpacks", [])
        settings = data.get("settings", [])

        spec = {}
        for setting in settings:
            if not isinstance(setting, dict):
                raise TypeError(
                    f"Each setting must be a dict, got {type(setting).__name__}: {setting!r}"
                )
            k = setting["key"]
            v = setting["value"]
            type = setting["type"]
            cast = self._cast_value(v, type)
            spec[k] = cast

        spec["versions"] = []
        for modpack in modpacks:
            if not isinstance(modpack, dict):
                raise TypeError(
                    f"Each modpack must be a dict, got {type(modpack).__name__}: {modpack!r}"
                )

            id = modpack.pop("id")
            print(f"id[{id}] added to spec")
            modpack["include"] = [
                {"path": "mods", "overwrite": True},
                {"path": "config", "overwrite": False},
            ]
            modpack["include_from"] = f"{config.MODPACKS_SAVES_DIR}/{id}"
            loader_name = modpack.pop("loader")
            modpack["loader_name"] = loader_name
            spec["versions"].append(modpack)

        save_spec_file(spec)

    def _cast_value(self, value: str, type: str) -> str | bool:
        if type == "boolean":
            return bool(value)
        return str(value)

    def update(self, modpack: Modpack) -> Modpack:
        data = read_file()
        modpacks = data.get("modpacks", [])

        modpack_id = modpack.id
        for i, raw in enumerate(modpacks):
            if raw.get("id") == modpack_id:
                modpacks[i] = {
                    "id": raw.get("id"),
                    "name": modpack.name,
                    "minecraft_version": modpack.minecraft_version,
                    "loader": modpack.loader.value,
                    "loader_version": modpack.loader_version,
                    "auth_config": self._auth_as_dict(modpack),
                }
                data["modpacks"] = modpacks
                save_file(data)
                return Modpack(**modpacks[i])

        raise Exception("Modpack not found")

    def get_all(self) -> list[Modpack]:
        data = read_file()
        raw_modpacks = data.get("modpacks", [])
        return [Modpack(**raw) for raw in raw_modpacks]

    def delete(self, id: int) -> None:
        data = read_file()
        modpacks = data.get("modpacks", [])
        new_modpacks = [m for m in modpacks if m.get("id") != id]

        if len(new_modpacks) == len(modpacks):
            raise Exception(f"Modpack id {id} doesn't exist")

        data["modpacks"] = new_modpacks
        save_file(data)
