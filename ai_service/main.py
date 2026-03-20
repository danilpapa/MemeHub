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


_LLM_PIPE = None


def _get_llm_pipe():
    global _LLM_PIPE  # noqa: PLW0603
    if _LLM_PIPE is not None:
        return _LLM_PIPE

    try:
        import torch
        from transformers import AutoModelForCausalLM, AutoTokenizer, pipeline
    except Exception as exc:  # noqa: BLE001
        raise HTTPException(
            status_code=500,
            detail=(
                "LLM зависимости не установлены. Установи: "
                "pip install -r requirements.txt"
            ),
        ) from exc

    model_name = os.getenv("QWEN_MODEL", "Qwen/Qwen2.5-0.5B-Instruct")
    device = "mps" if torch.backends.mps.is_available() else "cpu"

    tokenizer = AutoTokenizer.from_pretrained(model_name, use_fast=True)
    model = AutoModelForCausalLM.from_pretrained(model_name)

    _LLM_PIPE = pipeline(
        "text-generation",
        model=model,
        tokenizer=tokenizer,
        device=device,
    )
    return _LLM_PIPE


def _llm_tags_emotion(text: str) -> Tuple[List[str], str]:
    # Пытаемся получить JSON от модели, при неудаче падаем на заглушку.
    if not text.strip():
        return _simple_tags_emotion(text)

    pipe = _get_llm_pipe()
    prompt = (
        "Ты помощник для тегирования мемов. "
        "Верни ТОЛЬКО JSON без пояснений. "
        "Формат: {\"tags\": [\"tag1\", \"tag2\"], \"emotion\": \"joy|sadness|anger|neutral\"}.\n"
        f"Текст мема:\n{text}\n"
    )

    try:
        out = pipe(
            prompt,
            max_new_tokens=120,
            do_sample=False,
            temperature=0.0,
        )[0]["generated_text"]
    except Exception:
        return _simple_tags_emotion(text)

    # Вытаскиваем JSON из ответа.
    start = out.find("{")
    end = out.rfind("}")
    if start == -1 or end == -1 or end <= start:
        return _simple_tags_emotion(text)

    raw = out[start : end + 1]
    try:
        import json

        payload = json.loads(raw)
        tags = payload.get("tags") or []
        emotion = payload.get("emotion") or "neutral"
        if not isinstance(tags, list):
            tags = []
        if not isinstance(emotion, str):
            emotion = "neutral"
        if not tags:
            tags = ["meme"]
        return tags, emotion
    except Exception:
        return _simple_tags_emotion(text)


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
    use_llm = os.getenv("USE_LLM", "0") == "1"
    if use_llm:
        tags, emotion = _llm_tags_emotion(ocr_text)
    else:
        tags, emotion = _simple_tags_emotion(ocr_text)

    return ProcessResponse(ocr_text=ocr_text.strip(), tags=tags, emotion=emotion)


if __name__ == "__main__":
    # Для локального запуска: python3 main.py
    import uvicorn

    port = int(os.getenv("PORT", "8000"))
    uvicorn.run(app, host="0.0.0.0", port=port)
