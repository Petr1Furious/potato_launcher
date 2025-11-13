from fastapi import APIRouter, Depends
from app.utils.security import verify_access_token
from typing import List
from app.models.modpack import LoaderType

from app.services.mc_versions_service import (
    get_vanilla_versions,
    get_loaders_for_version,
    get_loader_versions,
)

router = APIRouter(
    prefix="/mc-versions",
    tags=["MC Versions"],
    dependencies=[Depends(verify_access_token)],
)


@router.get("", response_model=list[str], summary="List Vanilla MC Versions")
async def get() -> list[str]:
    versions = await get_vanilla_versions()
    return versions


@router.get(
    "/{version}/loaders",
    response_model=list[LoaderType],
    summary="List available loaders for version",
)
async def get(version: str) -> list[LoaderType]:
    loaders = await get_loaders_for_version(version)
    return loaders


@router.get(
    path="/{version}/{loader}",
    response_model=list[str],
    summary="List loader version for specified minecraft version",
)
async def get(version: str, loader: LoaderType) -> list[str]:
    loader_versions = await get_loader_versions(version, loader.value)
    return loader_versions
