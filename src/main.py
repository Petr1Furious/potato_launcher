import asyncio
import signal
import string
import sys
import traceback

import inquirer.errors
from rich import print
import httpx

from build_cfg import SERVER_BASE

from src import tui
from src.auth import AuthProvider
from src.auth.base import UnauthorizedException
from src.compat import perform_forbidden_nixery
from src.config import load_config, save_config, Config
from src.errors import LauncherError
from src.launcher import launch
from src.tui import ensure_tty, ask, clear
from src.update import update_if_required
from src.utils.java import find_java, ask_user_java
from src.utils.modpack import (
    get_modpack,
    sync_modpack,
    load_remote_indexes,
    load_local_indexes,
    ModpackIndex,
)


def validate_memory(mem: str):
    if not (mem and all(x in string.digits for x in mem)):
        raise inquirer.errors.ValidationError(mem, reason='Введите число')
    return True


def select_modpack(indexes: list[ModpackIndex]) -> str:
    if not indexes:
        raise LauncherError('Список сборок пуст')
    if len(indexes) == 1:
        return indexes[0].modpack_name
    return tui.choice(
        'Выберите сборку', [(x.modpack_name, x.modpack_name) for x in indexes]
    )


async def sync_and_launch(config: Config, online: bool):
    clear()
    print('Проверка версии...', flush=True)
    modpack_index = await get_modpack(config, online)
    if not modpack_index:
        location_msg = 'на сервере' if online else 'локально'
        print(
            f'\n[red]Ошибка! Сборка не найдена {location_msg}. Нажмите Enter чтобы выбрать сборку[/red]'
        )
        input()
        indexes = await load_remote_indexes() if online else load_local_indexes(config)
        config.modpack = select_modpack(indexes)
        modpack_index = await get_modpack(config, online)
        if not modpack_index:
            raise LauncherError('Сборка не найдена')

    if online:
        current_version = next(
            (
                x.modpack_version
                for x in load_local_indexes(config)
                if x.modpack_name == modpack_index.modpack_name
            ),
            None,
        )
        remote_version = modpack_index.modpack_version
        if not current_version or current_version != remote_version:
            await sync_modpack(config, modpack_index)

    print('[green]Запуск![/green]', flush=True)
    await launch(modpack_index, config, online)


async def main_menu(indexes: list[ModpackIndex], config: Config, online: bool):
    print('Загрузка...', end='', flush=True)
    while True:
        clear()
        online_msg = '[green](онлайн)[/green]' if online else '[red](офлайн)[/red]'
        print(f'Вы вошли как [green]{config.user_info.username}[/green] ' + online_msg)
        
        print(f'[red]Эта версия лаунчера устарела. Скачайте новый лаунчер по ссылке[/red] [bright_cyan]{SERVER_BASE}launcher/[/bright_cyan]')

        select_modpack_entry = (
            [(f'Изменить сборку (выбрана {config.modpack})', 'change_modpack')]
            if len(indexes) > 1
            else []
        )

        sync_modpack_entry = (
            [('Синхронизировать сборку', 'sync_modpack')] if online else []
        )

        selected_modpack_index = next(
            (x for x in indexes if x.modpack_name == config.modpack), None
        )
        if not selected_modpack_index:
            raise ValueError('Selected modpack not found in indexes')

        required_java_version = selected_modpack_index.java_version
        if not (
            config.modpack in config.java_path and config.java_path[config.modpack]
        ):
            config.java_path[config.modpack] = await find_java(required_java_version, config)
            save_config(config)
            continue
        java_path = config.java_path[config.modpack]

        answer = tui.choice(
            'Выберите опцию',
            [
                ('Играть', 'start'),
                *select_modpack_entry,
                *sync_modpack_entry,
                (f'Путь к Java ({java_path or "Не задан"})', 'java_path'),
                (f'Выделенная память ({config.xmx} МиБ)', 'xmx'),
                (
                    f'Путь к данным игры ({config.data_dir or "По умолчанию"})',
                    'data_dir',
                ),
                (
                    f'Путь к ассетам ({config.assets_dir or "По умолчанию"})',
                    'assets_dir',
                ),
                ('Выход', 'exit'),
            ],
        )
        if answer == 'start':
            await sync_and_launch(config, online)
            break
        elif answer == 'change_modpack':
            config.modpack = select_modpack(indexes)
        elif answer == 'sync_modpack':
            force_overwrite = tui.choice(
                'Сбросить опциональные файлы (конфиги и подобное)?',
                [('Нет', False), ('Да', True)],
            )
            await sync_modpack(config, selected_modpack_index, force_overwrite)
        elif answer == 'java_path':
            config.java_path[config.modpack] = ask_user_java(
                required_java_version, java_path
            ).path
        elif answer == 'xmx':
            config.xmx = int(
                ask(
                    'Выделенная память',
                    validate=validate_memory,
                    default=str(config.xmx),
                )
            )
        elif answer == 'data_dir':
            config.data_dir = ask(
                'Путь к данным игры',
                default=str(config.data_dir),
            )
        elif answer == 'assets_dir':
            config.assets_dir = ask(
                'Путь к ассетам',
                default=str(config.assets_dir),
            )
        elif answer == 'exit':
            break
        save_config(config)


async def _main():
    await update_if_required()
    ensure_tty()
    config = load_config()

    online = True

    auth_provider = AuthProvider.get()
    if not config.token:
        try:
            config.token = await auth_provider.authenticate()
        except httpx.HTTPError:
            raise LauncherError('При первом запуске необходим доступ к интернету')
    try:
        config.user_info = await auth_provider.get_user(config.token)
        save_config(config)
    except UnauthorizedException:
        config.token = await auth_provider.authenticate()
        config.user_info = await auth_provider.get_user(config.token)
        save_config(config)
    except httpx.HTTPError:
        online = False

    if online:
        try:
            indexes = await load_remote_indexes()
        except httpx.HTTPError:
            online = False

    if not online:
        indexes = load_local_indexes(config)

    if not indexes:
        if online:
            raise LauncherError('Список сборок пуст')
        else:
            raise LauncherError('Не удалось загрузить список сборок')

    if not config.modpack or not any(x.modpack_name == config.modpack for x in indexes):
        config.modpack = select_modpack(indexes)
        save_config(config)

    perform_forbidden_nixery()

    if '--launch' in sys.argv:
        await sync_and_launch(config, online)
    else:
        await main_menu(indexes, config, online)


def sigint_handler(signum, frame):
    print("\n[blue]Выход...[/blue]")
    sys.exit(0)


def main():
    signal.signal(signal.SIGINT, sigint_handler)

    try:
        asyncio.run(_main())
    except LauncherError as e:
        print(f'[red]{e.message}[/red]')
        print('[blue]Нажмите Enter чтобы выйти[/blue]')
        input()
    except Exception:
        traceback.print_exc()
        print('[red]Произошла неизвестная ошибка[/red]')
        print('[blue]Нажмите Enter чтобы выйти[/blue]')
        input()
