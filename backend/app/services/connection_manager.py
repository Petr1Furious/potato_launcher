from starlette.websockets import WebSocket


class ConnectionManager:

    def __init__(self):
        self.connections: list[WebSocket] = []

    async def connect(self, websocket: WebSocket):
        await websocket.accept()
        self.connections.append(websocket)

    async def disconnect(self, websocket: WebSocket):
        if websocket in self.connections:
            self.connections.remove(websocket)

    async def notify_all(self, text: str):
        to_remove = []
        for connection in self.connections:
            try:
                await connection.send_text(text)
            except Exception:
                to_remove.append(connection)

        if to_remove:
            self.connections = [c for c in self.connections if c not in to_remove]
