from __future__ import annotations
import asyncio
import json
from pathlib import Path
from typing import Any, Optional
from urllib.parse import urlparse

from fastapi import HTTPException

from app.models.modpack import LoaderType, Modpack
from app.models.setting import Setting
from app.services.connection_manager import ConnectionManager
from app.services.mc_versions_service import (
    get_loader_versions,
    get_loaders_for_version,
    get_vanilla_versions,
)
from app.config import config


class RunnerService:
    def __init__(self, connection_manager: ConnectionManager) -> None:
        self._busy: bool = False
        self._lock = asyncio.Lock()
        self._connection_manager = connection_manager
        self._message: Optional[str] = None

    async def is_running(self) -> bool:
        async with self._lock:
            return self._busy

    async def run_build(self) -> bool:
        await self._validate_spec(config.SPEC_FILE)

        should_notify = False
        async with self._lock:
            if self._busy:
                raise HTTPException(
                    status_code=409, detail="Another build is already running"
                )
            self._busy = True
            self._message = "running"
            should_notify = True

            self._task = asyncio.create_task(self._execute_instance_builder())

        if should_notify:
            await self._broadcast_status()

        return True

    async def _execute_instance_builder(self) -> None:
        cmd = [
            config.INSTANCE_BUILDER_BINARY,
            "-s",
            str(config.SPEC_FILE),
            str(config.GENERATED_DIR),
            str(config.WORKDIR_DIR),
        ]

        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                # stdout=asyncio.subprocess.PIPE,
                # stderr=asyncio.subprocess.PIPE,
            )

            # TODO log stdout/stderr here
            stdout, stderr = await proc.communicate()
            print(f"build finised\n")

            msg = "ok" if proc.returncode == 0 else f"failed (code {proc.returncode})"

            async with self._lock:
                self._message = msg
                self._busy = False
            await self._broadcast_status()
        except FileNotFoundError as exc:
            async with self._lock:
                self._message = f"command not found: {exc}"
                self._busy = False
            await self._broadcast_status()
            print(f"Error on build: {self._message}")
        except Exception as exc:
            async with self._lock:
                self._message = f"failed: {exc}"
                self._busy = False
            await self._broadcast_status()
            print(f"Error on build: {self._message}")

    async def _validate_spec(self, spec_path: Path) -> None:
        try:
            raw_text = spec_path.read_text(encoding="utf-8")
        except FileNotFoundError as exc:
            raise HTTPException(status_code=404, detail="Spec file not found") from exc
        except OSError as exc:
            raise HTTPException(
                status_code=500, detail=f"Unable to read spec file: {exc}"
            ) from exc

        try:
            raw = json.loads(raw_text)
        except json.JSONDecodeError as exc:
            raise HTTPException(
                status_code=422, detail=f"Spec file must contain valid JSON: {exc}"
            ) from exc

        if not isinstance(raw, dict):
            raise HTTPException(
                status_code=422, detail="Spec file must be a JSON object"
            )

        required_root_fields = [
            "download_server_base",
            "resources_url_base",
            "version_manifest_url",
            "versions",
        ]

        for key in required_root_fields:
            if key not in raw:
                raise HTTPException(
                    status_code=422, detail=f"Field '{key}' is required"
                )

        if not isinstance(raw["versions"], list) or not raw["versions"]:
            raise HTTPException(
                status_code=422, detail="Field 'versions' must be a non-empty list"
            )

        for url_key in [
            "download_server_base",
            "resources_url_base",
            "version_manifest_url",
        ]:
            self._ensure_url(raw[url_key], url_key)

        try:
            vanilla_versions = set(await get_vanilla_versions())
        except Exception as exc:
            raise HTTPException(
                status_code=503, detail=f"Failed to fetch vanilla versions: {exc}"
            ) from exc

        version_required_fields = [
            "name",
            "minecraft_version",
            "loader_name",
            "loader_version",
            "include_from",
        ]

        for idx, version_cfg in enumerate(raw["versions"]):
            if not isinstance(version_cfg, dict):
                raise HTTPException(
                    status_code=422, detail=f"versions[{idx}] must be a JSON object"
                )

            for field in version_required_fields:
                if field not in version_cfg:
                    raise HTTPException(
                        status_code=422,
                        detail=f"Field 'versions[{idx}].{field}' is required",
                    )
                value = version_cfg[field]
                if not isinstance(value, str) or not value.strip():
                    raise HTTPException(
                        status_code=422,
                        detail=f"Field 'versions[{idx}].{field}' must be a non-empty string",
                    )

            minecraft_version = version_cfg["minecraft_version"]
            loader_name_raw = version_cfg["loader_name"].lower()

            if minecraft_version not in vanilla_versions:
                raise HTTPException(
                    status_code=422,
                    detail=f"versions[{idx}].minecraft_version '{minecraft_version}' is not a known Minecraft version",
                )

            try:
                loader_type = LoaderType(loader_name_raw)
            except ValueError as exc:
                allowed = ", ".join([lt.value for lt in LoaderType])
                raise HTTPException(
                    status_code=422,
                    detail=f"versions[{idx}].loader_name must be one of: {allowed}",
                ) from exc

            try:
                loaders_for_version = await get_loaders_for_version(minecraft_version)
            except Exception as exc:
                raise HTTPException(
                    status_code=503,
                    detail=f"Failed to determine loaders for Minecraft {minecraft_version}: {exc}",
                ) from exc

            if loader_type not in loaders_for_version:
                raise HTTPException(
                    status_code=422,
                    detail=(
                        f"versions[{idx}].loader_name '{loader_type.value}' is not available "
                        f"for Minecraft {minecraft_version}"
                    ),
                )

            try:
                loader_versions = await get_loader_versions(
                    minecraft_version, loader_type.value
                )
            except Exception as exc:
                raise HTTPException(
                    status_code=503,
                    detail=(
                        f"Failed to fetch loader versions for Minecraft {minecraft_version} "
                        f"and loader {loader_type.value}: {exc}"
                    ),
                ) from exc

            loader_version = version_cfg["loader_version"]
            if loader_version not in loader_versions:
                raise HTTPException(
                    status_code=422,
                    detail=(
                        f"versions[{idx}].loader_version '{loader_version}' is not available for "
                        f"Minecraft {minecraft_version} ({loader_type.value})"
                    ),
                )

    def _ensure_url(self, value: Any, field_name: str) -> None:
        if not isinstance(value, str) or not value.strip():
            raise HTTPException(
                status_code=422,
                detail=f"Field '{field_name}' must be a non-empty URL string",
            )

        parsed = urlparse(value)
        if parsed.scheme not in {"http", "https"} or not parsed.netloc:
            raise HTTPException(
                status_code=422,
                detail=f"Field '{field_name}' must be a valid HTTP or HTTPS URL",
            )

    async def _broadcast_status(self) -> None:
        if not self._connection_manager.connections:
            return

        message = self._busy
        try:
            await self._connection_manager.notify_all(message)
        except Exception:
            pass
