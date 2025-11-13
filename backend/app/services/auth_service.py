from datetime import datetime, timedelta, timezone
from app.config import config
import jwt


def create_access_token(data: dict):
    expire = datetime.now(timezone.utc) + timedelta(
        minutes=config.ACCESS_TOKEN_EXPIRE_MINUTES
    )
    to_encode = {**data, "exp": expire}
    return jwt.encode(to_encode, config.ADMIN_JWT_SECRET, algorithm=config.ALGORITHM)
