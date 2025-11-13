from typing import List, Optional
from app.models.modpack import LoaderType

from app.integrations.vanilla_versions import get_vanilla_version_list
from app.integrations.forge_loader import (
    forge_has_loader_for,
    get_forge_loader_versions,
)
from app.integrations.neoforge_loader import (
    neoforge_has_loader_for,
    get_neoforge_loader_versions,
)
from app.integrations.fabric_loader import (
    fabric_has_loader_for,
    get_fabric_loader_versions,
)
from async_lru import alru_cache


@alru_cache
async def get_vanilla_versions(version_type: Optional[str] = None) -> List[str]:
    return await get_vanilla_version_list(version_type)


@alru_cache
async def get_loaders_for_version(version: str) -> List[str]:
    loaders: List[LoaderType] = []

    vanilla_versions = await get_vanilla_versions()
    if version in vanilla_versions:
        loaders.append(LoaderType.VANILLA)

    if await fabric_has_loader_for(version):
        loaders.append(LoaderType.FABRIC)

    if await forge_has_loader_for(version):
        loaders.append(LoaderType.FORGE)

    if await neoforge_has_loader_for(version):
        loaders.append(LoaderType.NEOFORGE)

    return loaders


@alru_cache
async def get_loader_versions(version: str, loader: str) -> list[str]:
    loader = loader.lower()

    if loader == "vanilla":
        return [version]

    elif loader == "fabric":
        return await get_fabric_loader_versions(version)

    elif loader == "forge":
        return await get_forge_loader_versions(version)

    elif loader == "neoforge":
        return await get_neoforge_loader_versions(version)

    else:
        return []
