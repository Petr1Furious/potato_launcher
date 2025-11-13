from __future__ import annotations
from typing import List, Optional, Any
import httpx

MOJANG_MANIFEST_URL = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"


async def _fetch_mojang_manifest() -> dict[str, Any]:
    async with httpx.AsyncClient(timeout=10.0) as client:
        resp = await client.get(MOJANG_MANIFEST_URL)
        resp.raise_for_status()
        return resp.json()


async def get_vanilla_version_list(version_type: Optional[str] = None) -> List[str]:
    manifest = await _fetch_mojang_manifest()
    versions = manifest.get("versions", [])

    result: List[str] = []
    for v in versions:
        vid = v.get("id")
        vtype = v.get("type")
        if not vid:
            continue

        if version_type is None or vtype == version_type:
            result.append(vid)

    return result
