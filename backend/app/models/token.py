from app.models.base import ApiModel


class TokenRequest(ApiModel):
    token: str
