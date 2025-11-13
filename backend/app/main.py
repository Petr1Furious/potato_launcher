import uvicorn
from fastapi import FastAPI, APIRouter
from starlette import status
from starlette.middleware.cors import CORSMiddleware
from starlette.requests import Request
from starlette.responses import JSONResponse

from app.config import config
from app.gateway.json.modpack import JsonModpackGateway
from app.gateway.json.setting import JsonSettingGateway
from app.gateway.modpack import ModpackGateway
from app.gateway.settings import SettingGateway
from app.routers import auth, ws
from app.routers import modpacks
from app.routers import mc_versions
from app.routers import settings
from app.services.connection_manager import ConnectionManager
from app.services.runner_service import RunnerService


async def handle_error(_: Request, exc: Exception) -> JSONResponse:
    return JSONResponse(
        status_code=status.HTTP_500_INTERNAL_SERVER_ERROR, content={"details": str(exc)}
    )


def catch_exceptions(app: FastAPI) -> None:
    app.add_exception_handler(ValueError, handle_error)
    app.add_exception_handler(Exception, handle_error)


def register_routers(app: FastAPI) -> None:
    api_v1_router = APIRouter(prefix="/api/v1")

    api_v1_router.include_router(auth.router)
    api_v1_router.include_router(modpacks.router)
    api_v1_router.include_router(mc_versions.router)
    api_v1_router.include_router(settings.router)
    api_v1_router.include_router(ws.ws_router)

    app.include_router(api_v1_router)


def create_app() -> FastAPI:
    app = FastAPI()

    app.add_middleware(
        CORSMiddleware,
        allow_origins=config.ALLOWED_ORIGINS,
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

    connection_manager = ConnectionManager()
    runner = RunnerService(connection_manager)
    app.dependency_overrides[SettingGateway] = lambda: JsonSettingGateway()
    app.dependency_overrides[ModpackGateway] = lambda: JsonModpackGateway()
    app.dependency_overrides[ConnectionManager] = lambda: connection_manager
    app.dependency_overrides[RunnerService] = lambda: runner

    register_routers(app)
    catch_exceptions(app)

    return app


if __name__ == "__main__":
    app = create_app()
    uvicorn.run(app=app, host=config.HOST, port=config.PORT)
