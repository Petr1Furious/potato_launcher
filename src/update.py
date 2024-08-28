import os
from hashlib import sha1
from pathlib import Path

import httpx
import sys
import errno

from rich import print
from rich.progress import Progress

from build_cfg import SERVER_BASE, LAUNCHER_NAME
from src.compat import iswin, ismac, islinux, is_frozen, chmod_x
from src.errors import LauncherError


def get_update_url():
    if iswin():
        return f'{SERVER_BASE}/launcher/{LAUNCHER_NAME}.exe'
    if ismac():
        return f'{SERVER_BASE}/launcher/{LAUNCHER_NAME}_macos'
    if islinux():
        return f'{SERVER_BASE}/launcher/{LAUNCHER_NAME}_linux'
    raise ValueError('Unsupported platform')


async def update_required():
    if not is_frozen():
        return False
    print('Проверка обновлений...', flush=True)
    hash_url = get_update_url() + '.sha1'
    client = httpx.AsyncClient()
    new_hash_resp = await client.get(hash_url)
    new_hash = new_hash_resp.text.strip()
    with open(sys.executable, 'rb') as f:
        current_hash = sha1(f.read()).hexdigest()
    return new_hash != current_hash


async def download_update(file_path: Path):
    client = httpx.AsyncClient()
    with open(file_path, 'wb') as f:
        async with client.stream('GET', get_update_url()) as resp:
            total = int(resp.headers['Content-Length'])
            with Progress() as progress:
                t = progress.add_task('Обновление...', total=total)
                async for chunk in resp.aiter_bytes():
                    f.write(chunk)
                    progress.update(t, completed=resp.num_bytes_downloaded)


async def update_if_required():
    return
    try:
        if len(sys.argv) == 3 and sys.argv[1] == 'updated':
            Path(sys.argv[2]).unlink()
            return
        if not await update_required():
            return

        current_file = Path(sys.executable)
        old_file_name = current_file.with_stem(current_file.stem + '_old')

        if iswin():
            # windows defender doesn't like renaming a file immediately
            # after downloading, so downloading directly to final path
            current_file.rename(old_file_name)
            await download_update(current_file)
        else:
            # using safer approach on sane oses
            upd_file_name = current_file.with_stem(current_file.stem + '_upd')
            await download_update(upd_file_name)
            current_file.rename(old_file_name)
            upd_file_name.rename(current_file)

        chmod_x(current_file)
        os.execl(sys.executable, sys.executable, 'updated', old_file_name)
    except OSError as e:
        if e.errno == errno.EROFS:
            message = 'Не удалось обновить лаунчер, недостаточно прав на запись'
            if ismac():
                message += '. Если файл запущен из образа диска, перетащите его оттуда (например, в папку Applications)'
            raise LauncherError(message)
        else:
            raise


__all__ = ['update_if_required']
