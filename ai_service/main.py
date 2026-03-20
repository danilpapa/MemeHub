from __future__ import annotations

import io
import os
from typing import List, Optional, Tuple

import httpx
import pytesseract
from fastapi import FastAPI, File, HTTPException, UploadFile
from pydantic import BaseModel, HttpUrl
from PIL import Image

app = FastAPI(title="MemeHub AI Processing Service", version="0.1.0")


class ProcessRequest(BaseModel):
    image_url: HttpUrl


class ProcessResponse(BaseModel):
    ocr_text: str
    tags: List[str]
    emotion: str


def _ocr_image(image: Image.Image) -> str:
    try:
        return pytesseract.image_to_string(image) or ""
    except pytesseract.TesseractNotFoundError as exc:
        raise HTTPException(
            status_code=500,
            detail=(
                "Tesseract не установлен. Установи через Homebrew: "
                "brew install tesseract"
            ),
        ) from exc


def _simple_tags_emotion(text: str) -> Tuple[List[str], str]:
    # Заглушка: минимальная эвристика, чтобы API работал.
    text_l = text.lower()
    tags: List[str] = []

    if any(k in text_l for k in ["lol", "lmao", "аха", "ахах", "хаха"]):
        tags.append("humor")
    if any(k in text_l for k in ["sad", "печаль", "грусть"]):
        tags.append("sad")
    if any(k in text_l for k in ["angry", "злюсь", "бесит"]):
        tags.append("angry")

    if not tags:
        tags.append("meme")

    # Эмоция — одна строка.
    emotion = "neutral"
    if "humor" in tags:
        emotion = "joy"
    elif "sad" in tags:
        emotion = "sadness"
    elif "angry" in tags:
        emotion = "anger"

    return tags, emotion


def _load_image_from_bytes(data: bytes) -> Image.Image:
    try:
        return Image.open(io.BytesIO(data)).convert("RGB")
    except Exception as exc:  # noqa: BLE001 - широкая обработка для чистого API
        raise HTTPException(status_code=400, detail="Некорректное изображение") from exc


async def _fetch_image(url: str) -> Image.Image:
    timeout = httpx.Timeout(10.0, read=20.0)
    async with httpx.AsyncClient(timeout=timeout, follow_redirects=True) as client:
        resp = await client.get(url)
        if resp.status_code >= 400:
            raise HTTPException(status_code=400, detail="Не удалось скачать изображение")
        return _load_image_from_bytes(resp.content)


@app.get("/health")
def health() -> dict:
    return {"status": "ok"}


@app.post("/process", response_model=ProcessResponse)
async def process(
    file: Optional[UploadFile] = File(default=None),
    body: Optional[ProcessRequest] = None,
) -> ProcessResponse:
    if file is None and body is None:
        raise HTTPException(status_code=400, detail="Нужен file или image_url")

    if file is not None:
        data = await file.read()
        image = _load_image_from_bytes(data)
    else:
        image = await _fetch_image(str(body.image_url))

    ocr_text = _ocr_image(image)
    tags, emotion = _simple_tags_emotion(ocr_text)

    return ProcessResponse(ocr_text=ocr_text.strip(), tags=tags, emotion=emotion)


if __name__ == "__main__":
    # Для локального запуска: python3 main.py
    import uvicorn

    port = int(os.getenv("PORT", "8000"))
    uvicorn.run(app, host="0.0.0.0", port=port)
