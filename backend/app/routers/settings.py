from fastapi import APIRouter, Depends, Body
from typing import Annotated

from app.gateway.settings import SettingGateway
from app.models.setting import SettingResponse, UpdateSettingRequest
from app.utils.security import verify_access_token
from app.utils.stub import Stub

router = APIRouter(
    prefix="/settings", tags=["Settings"], dependencies=[Depends(verify_access_token)]
)


@router.get(path="", summary="List settings", response_model=list[SettingResponse])
def get_settings(
    setting_gateway: Annotated[SettingGateway, Depends(Stub(SettingGateway))],
):
    return setting_gateway.get_all()


@router.post(
    path="", summary="Create or update settings", response_model=list[SettingResponse]
)
def create_settings(
    setting_gateway: Annotated[SettingGateway, Depends(Stub(SettingGateway))],
    body: list[UpdateSettingRequest] = Body(),
):
    settings = [setting.to_model() for setting in body]
    return setting_gateway.update(settings)
