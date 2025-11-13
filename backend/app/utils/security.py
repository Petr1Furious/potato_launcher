import jwt
from fastapi import HTTPException, Depends
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from starlette import status

from app.config import config

creds = HTTPBearer()


def validate_access_token(token: str):
    try:
        payload = jwt.decode(
            token, config.ADMIN_JWT_SECRET, algorithms=[config.ALGORITHM]
        )
        if payload.get("sub") != "single_user":
            raise HTTPException(
                status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid JWT payload"
            )
        return payload
    except jwt.ExpiredSignatureError:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Token expired"
        )
    except jwt.PyJWTError:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid JWT"
        )


def verify_access_token(credentials: HTTPAuthorizationCredentials = Depends(creds)):
    validate_access_token(token=credentials.credentials)
