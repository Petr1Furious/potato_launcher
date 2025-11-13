import os
import shutil
import traceback
from pathlib import Path, PurePosixPath
from typing import Annotated, Literal

import aiofiles
from fastapi import APIRouter, Depends, HTTPException, UploadFile, File
from starlette import status

from app.config import config
from app.gateway.modpack import ModpackGateway
from app.models.modpack import (
    ModpackResponse,
    CreateModpackRequest,
    UpdateModpackRequest,
)
from app.services.runner_service import RunnerService
from app.utils.security import verify_access_token
from app.utils.stub import Stub

router = APIRouter(
    prefix="/modpacks", tags=["Modpacks"], dependencies=[Depends(verify_access_token)]
)


@router.get(path="", summary="List modpacks", response_model=list[ModpackResponse])
def get(modpack_gateway: Annotated[ModpackGateway, Depends(Stub(ModpackGateway))]):
    return modpack_gateway.get_all()


@router.post("", summary="Create new modpack", response_model=ModpackResponse)
def get(
    modpack_gateway: Annotated[ModpackGateway, Depends(Stub(ModpackGateway))],
    body: CreateModpackRequest,
):
    return modpack_gateway.save(body.to_model())


@router.get(
    "/{id}", summary="Get info about the modpack", response_model=ModpackResponse
)
def get(
    modpack_gateway: Annotated[ModpackGateway, Depends(Stub(ModpackGateway))], id: int
):
    modpack = modpack_gateway.get_by_id(id)
    if modpack is None:
        raise HTTPException(status_code=404, detail="Modpack not found")
    return modpack


@router.patch("/{id}", summary="Edit modpack")
def get(
    id: int,
    body: UpdateModpackRequest,
    modpack_gateway: Annotated[ModpackGateway, Depends(Stub(ModpackGateway))],
):
    return modpack_gateway.update(body.as_model(id))


@router.delete(
    "/{id}", summary="Delete modpack", status_code=status.HTTP_204_NO_CONTENT
)
def get(
    id: int,
    modpack_gateway: Annotated[ModpackGateway, Depends(Stub(ModpackGateway))],
):
    modpack = modpack_gateway.get_by_id(id)
    if modpack is None:
        raise HTTPException(status_code=404, detail="Modpack not found")
    modpack_gateway.delete(id)


@router.post(
    "/build",
    summary="Run build",
)
async def build(
    runner_service: Annotated[RunnerService, Depends(Stub(RunnerService))],
    modpack_gateway: Annotated[ModpackGateway, Depends(Stub(ModpackGateway))],
):
    if len(modpack_gateway.get_all()) == 0:
        raise HTTPException(status_code=400, detail="Modpacks should be more than 0")

    modpack_gateway.generate_spec()
    is_success = await runner_service.run_build()
    if not is_success:
        raise HTTPException(status_code=400, detail="Failed to build modpacks")
    return True


@router.get(
    "/build/status",
    summary="Get build status",
    response_model=Literal["running", "idle"],
)
async def get_build_status(
    runner_service: Annotated[RunnerService, Depends(Stub(RunnerService))],
):
    is_running = await runner_service.is_running()
    return "running" if is_running else "idle"


def sanitize_relative(p: str) -> str:
    norm = os.path.normpath(p.lstrip("/\\"))
    if norm.startswith("..") or os.path.isabs(norm):
        raise HTTPException(status_code=400, detail="Invalid relative path")
    return norm


def strip_top_folder(p: str) -> str:
    parts = PurePosixPath(p.replace("\\", "/")).parts
    return "/".join(parts[1:]) if len(parts) > 1 else parts[0]


@router.post(path="/{id}/files", summary="Upload modpack files")
async def upload_modpack_files(
    id: int,
    modpack_gateway: Annotated[ModpackGateway, Depends(Stub(ModpackGateway))],
    files: list[UploadFile] = File(...),
):
    if not files:
        raise HTTPException(status_code=400, detail="No modpack files to upload")

    modpack = modpack_gateway.get_by_id(id)
    if not modpack:
        raise HTTPException(status_code=404, detail="Modpack not found")

    tmp_dir = config.TEMP_DIR.joinpath(str(id))
    final_dir = config.MODPACKS_SAVES_DIR.joinpath(str(id))

    try:
        tmp_dir.mkdir(parents=True, exist_ok=True)

        for number, file in enumerate(files):
            rel = (
                sanitize_relative(
                    file.filename or file.filename == "" and file.filename
                )
                if file.filename
                else None
            )
            if not rel:
                rel = sanitize_relative(Path(file.filename or f"file_{number}").name)

            rel = strip_top_folder(rel)

            dst_path = tmp_dir.joinpath(rel)
            dst_path.parent.mkdir(parents=True, exist_ok=True)

            async with aiofiles.open(dst_path, "wb") as out:
                while True:
                    chunk = await file.read(1024 * 1024)
                    if not chunk:
                        break
                    await out.write(chunk)
            await file.close()

        if final_dir.exists():
            shutil.rmtree(final_dir)

        shutil.copytree(tmp_dir, final_dir)
    except Exception:
        raise HTTPException(status_code=500, detail="Failed to upload modpack files")
    finally:
        if tmp_dir.exists():
            shutil.rmtree(tmp_dir, ignore_errors=True)
