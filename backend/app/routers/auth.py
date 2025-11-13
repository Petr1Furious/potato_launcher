from fastapi import APIRouter, HTTPException
from fastapi.security import HTTPBearer

from app.models.token import TokenRequest
from app.services.auth_service import create_access_token
from app.config import config

router = APIRouter(prefix="/auth", tags=["Authorization"])
security = HTTPBearer()


@router.post("/login")
def auth(request: TokenRequest):
    if request.token != config.ADMIN_SECRET_TOKEN:
        raise HTTPException(status_code=401, detail="Invalid token")

    jwt_token = create_access_token({"sub": "single_user"})
    return {"access_token": jwt_token, "token_type": "bearer"}
