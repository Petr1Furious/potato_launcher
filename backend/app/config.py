from pathlib import Path

from pydantic_settings import BaseSettings

ROOT = Path(__file__).resolve().parent.parent


class ConfigEnv(BaseSettings):
    ADMIN_JWT_SECRET: str
    ADMIN_SECRET_TOKEN: str
    ACCESS_TOKEN_EXPIRE_MINUTES: int = 1440
    ALGORITHM: str = "HS256"
    HOST: str = "0.0.0.0"
    PORT: int = 8000
    ALLOWED_ORIGINS: list[str] = ["*"]
    TEMP_DIR: Path = Path("/tmp")
    MODPACKS_SAVES_DIR: Path = Path("/modpacks-saves")
    SPEC_FILE: Path = Path("/spec.json")
    INSTANCE_BUILDER_BINARY: str = "instance_builder"
    GENERATED_DIR: Path = Path("/generated")
    WORKDIR_DIR: Path = Path("/workdir")


config = ConfigEnv()
