from __future__ import annotations
from typing import Any, Dict, List
import httpx

FORGE_MAVEN_METADATA_URL = (
    "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json"
)

# TODO for recommended & latest description
FORGE_PROMOTIONS_URL = (
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json"
)


async def _fetch_forge_metadata() -> Dict[str, Any]:
    async with httpx.AsyncClient(timeout=10.0) as client:
        resp = await client.get(FORGE_MAVEN_METADATA_URL)
        resp.raise_for_status()
        return resp.json()


async def forge_has_loader_for(mc_version: str) -> bool:
    data = await _fetch_forge_metadata()
    versions_for_mc = data.get(mc_version)
    return bool(versions_for_mc)


def _version_key(v: str) -> List[int]:
    parts = []
    for x in v.split("."):
        try:
            parts.append(int(x))
        except ValueError:
            parts.append(0)
    return parts


async def get_forge_loader_versions(mc_version: str) -> List[str]:
    data = await _fetch_forge_metadata()
    versions_for_mc = data.get(mc_version, [])
    result: List[str] = []

    prefix = f"{mc_version}-"

    for full_ver in versions_for_mc:
        if full_ver.startswith(prefix):
            build = full_ver[len(prefix) :]
            result.append(build)
        else:
            result.append(full_ver)

    result = sorted(set(result), key=_version_key, reverse=True)
    return result
