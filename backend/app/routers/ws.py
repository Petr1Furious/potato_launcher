import asyncio
from typing import Annotated

from fastapi import APIRouter
from fastapi.params import Depends, Query
from starlette.websockets import WebSocket, WebSocketDisconnect

from app.services.connection_manager import ConnectionManager
from app.utils.security import validate_access_token
from app.utils.stub import Stub

ws_router = APIRouter(prefix="/ws", include_in_schema=False)


@ws_router.websocket("")
async def websocket_endpoint(
    websocket: WebSocket,
    manager: Annotated[ConnectionManager, Depends(Stub(ConnectionManager))],
):
    try:
        token = websocket.query_params.get("token")
        validate_access_token(token)
    except Exception:
        await websocket.close(code=1000, reason="Invalid token")
        return

    await manager.connect(websocket)

    try:
        while True:
            await websocket.receive()
    except Exception:
        pass
    finally:
        await manager.disconnect(websocket)
