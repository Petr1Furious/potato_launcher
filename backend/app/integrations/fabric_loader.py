from __future__ import annotations
from typing import List
import httpx

FABRIC_META_BASE_URL = "https://meta.fabricmc.net/v2/versions/loader/"


async def _fetch_fabric_meta(mc_version: str) -> List[dict]:
    url = f"{FABRIC_META_BASE_URL}{mc_version}"
    async with httpx.AsyncClient(timeout=10.0) as client:
        resp = await client.get(url)
        if resp.status_code == 404:
            return []
        elif resp.status_code == 400:
            return []
        resp.raise_for_status()
        return resp.json()


async def fabric_has_loader_for(mc_version: str) -> bool:
    items = await _fetch_fabric_meta(mc_version)
    return len(items) > 0


async def get_fabric_loader_versions(mc_version: str) -> List[str]:
    items = await _fetch_fabric_meta(mc_version)
    versions: List[str] = []
    for item in items:
        loader = item.get("loader")
        if not loader:
            continue
        ver = loader.get("version")
        if ver:
            versions.append(ver)

    seen = set()
    result: List[str] = []
    for v in versions:
        if v not in seen:
            seen.add(v)
            result.append(v)
    return result
