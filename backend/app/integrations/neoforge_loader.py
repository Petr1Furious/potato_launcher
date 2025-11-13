from __future__ import annotations
from typing import List
import httpx
import xml.etree.ElementTree as ET

NEOFORGE_MAVEN_METADATA_URL = (
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml"
)


async def _fetch_neoforge_versions() -> List[str]:
    async with httpx.AsyncClient(timeout=10.0) as client:
        resp = await client.get(NEOFORGE_MAVEN_METADATA_URL)
        resp.raise_for_status()
        xml_text = resp.text

    root = ET.fromstring(xml_text)
    versions_el = root.find("./versioning/versions")
    if versions_el is None:
        return []

    versions: List[str] = []
    for v in versions_el.findall("version"):
        if v.text:
            versions.append(v.text.strip())
    return versions


def _mc_to_neoforge_prefix(mc_version: str) -> str:
    # "1.21 -> 21.0."
    parts = mc_version.split(".")
    if len(parts) >= 3:
        # "1.20.4" -> "20.4."
        return f"{parts[1]}.{parts[2]}."
    elif len(parts) == 2:
        # "1.21" -> "21.0."
        return f"{parts[1]}.0."
    else:
        return ""


def _version_key_numeric(ver: str) -> List[int]:
    base = ver.split("-", 1)[0]
    nums = []
    for p in base.split("."):
        try:
            nums.append(int(p))
        except ValueError:
            nums.append(0)
    return nums


async def neoforge_has_loader_for(mc_version: str) -> bool:
    prefix = _mc_to_neoforge_prefix(mc_version)
    if not prefix:
        return False

    all_versions = await _fetch_neoforge_versions()
    return any(v.startswith(prefix) for v in all_versions)


async def get_neoforge_loader_versions(mc_version: str) -> List[str]:
    prefix = _mc_to_neoforge_prefix(mc_version)
    if not prefix:
        return []

    all_versions = await _fetch_neoforge_versions()
    matched = [v for v in all_versions if v.startswith(prefix)]

    stable = [v for v in matched if "-beta" not in v]
    beta = [v for v in matched if "-beta" in v]

    stable_sorted = sorted(stable, key=_version_key_numeric, reverse=True)
    beta_sorted = sorted(beta, key=_version_key_numeric, reverse=True)

    return stable_sorted + beta_sorted
