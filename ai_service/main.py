from __future__ import annotations

import io
import asyncio
import base64
import json
import os
import time
from typing import Any, List, Literal, Optional, Tuple, Union

from aiokafka import AIOKafkaConsumer, AIOKafkaProducer
import httpx
import pytesseract
from fastapi import FastAPI, File, HTTPException, UploadFile
from prometheus_fastapi_instrumentator import Instrumentator
from pydantic import BaseModel, Field, HttpUrl
from PIL import Image

app = FastAPI(title="MemeHub AI Processing Service", version="0.1.0")

Instrumentator(
    should_group_status_codes=False,
    should_ignore_untemplated=True,
    should_instrument_requests_inprogress=True,
).instrument(app).expose(app)

_CLICKHOUSE_URL = (
    f"http://{os.getenv('CLICKHOUSE_HOST', 'clickhouse')}:"
    f"{os.getenv('CLICKHOUSE_PORT', '8123')}"
)

_CREATE_EVENTS_TABLE = """
CREATE TABLE IF NOT EXISTS meme_events (
    job_id       String,
    tags         Array(String),
    emotion      String,
    ocr_length   UInt32,
    processing_ms UInt32,
    created_at   DateTime DEFAULT now()
) ENGINE = MergeTree()
ORDER BY created_at
"""


async def _init_clickhouse() -> None:
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            await client.post(_CLICKHOUSE_URL, content=_CREATE_EVENTS_TABLE.encode())
    except Exception:
        pass


async def _send_analytics(
    job_id: str,
    tags: List[str],
    emotion: str,
    ocr_len: int,
    processing_ms: int,
) -> None:
    try:
        row = json.dumps({
            "job_id": job_id,
            "tags": tags,
            "emotion": emotion,
            "ocr_length": ocr_len,
            "processing_ms": processing_ms,
        })
        async with httpx.AsyncClient(timeout=5.0) as client:
            await client.post(
                _CLICKHOUSE_URL,
                params={"query": "INSERT INTO meme_events FORMAT JSONEachRow"},
                content=row.encode(),
            )
    except Exception:
        pass


class ProcessRequest(BaseModel):
    image_url: HttpUrl


class ProcessResponse(BaseModel):
    ocr_text: str
    tags: List[str]
    emotion: str


class FileImageSource(BaseModel):
    type: Literal["file"]
    filename: Optional[str] = None
    content_type: Optional[str] = None
    data_base64: str


class UrlImageSource(BaseModel):
    type: Literal["url"]
    image_url: str


class AnalysisRequestedEvent(BaseModel):
    job_id: str
    image: Union[FileImageSource, UrlImageSource] = Field(discriminator="type")


class AnalysisCompletedEvent(BaseModel):
    job_id: str
    status: Literal["completed", "failed"]
    result: Optional[ProcessResponse] = None
    error: Optional[str] = None


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
        "Не переписывай текст мема и не возвращай цитаты из него. "
        "Нужны только короткие теги (1-3 слова), отражающие смысл, ситуацию, тип шутки, "
        "персонажей/контекст, если очевидно. "
        "Верни ТОЛЬКО JSON без пояснений. "
        "Формат: {\"tags\": [\"tag1\", \"tag2\"], \"emotion\": \"joy|sadness|anger|neutral\"}.\n"
        "Правила: максимум 6 тегов; теги без пунктуации; язык тегов — СТРОГО русский; "
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


async def _analyze_image(image: Image.Image) -> ProcessResponse:
    ocr_text = _ocr_image(image)
    use_llm = os.getenv("USE_LLM", "0") == "1"
    if use_llm:
        tags, emotion = _llm_tags_emotion(ocr_text)
    else:
        tags, emotion = _simple_tags_emotion(ocr_text)

    return ProcessResponse(ocr_text=ocr_text.strip(), tags=tags, emotion=emotion)


async def _image_from_event(source: Union[FileImageSource, UrlImageSource]) -> Image.Image:
    if isinstance(source, UrlImageSource):
        return await _fetch_image(source.image_url)

    try:
        data = base64.b64decode(source.data_base64)
    except Exception as exc:  # noqa: BLE001
        raise HTTPException(status_code=400, detail="Некорректный base64 файла") from exc

    return _load_image_from_bytes(data)


def _kafka_settings() -> tuple[str, str, str]:
    bootstrap_servers = os.getenv("KAFKA_BOOTSTRAP_SERVERS", "localhost:19092")
    requested_topic = os.getenv(
        "KAFKA_ANALYSIS_REQUESTED_TOPIC",
        "meme.analysis.requested",
    )
    completed_topic = os.getenv(
        "KAFKA_ANALYSIS_COMPLETED_TOPIC",
        "meme.analysis.completed",
    )
    return bootstrap_servers, requested_topic, completed_topic


async def _publish_completed(
    producer: AIOKafkaProducer,
    completed_topic: str,
    event: AnalysisCompletedEvent,
) -> None:
    payload = event.model_dump_json().encode("utf-8")
    await producer.send_and_wait(
        completed_topic,
        value=payload,
        key=event.job_id.encode("utf-8"),
    )


async def _consume_analysis_requests() -> None:
    bootstrap_servers, requested_topic, completed_topic = _kafka_settings()
    consumer = AIOKafkaConsumer(
        requested_topic,
        bootstrap_servers=bootstrap_servers,
        group_id="ai-service-analysis",
        enable_auto_commit=True,
        auto_offset_reset="earliest",
    )
    producer = AIOKafkaProducer(bootstrap_servers=bootstrap_servers)

    await consumer.start()
    await producer.start()
    try:
        async for message in consumer:
            try:
                payload: dict[str, Any] = json.loads(message.value.decode("utf-8"))
                event = AnalysisRequestedEvent.model_validate(payload)
                image = await _image_from_event(event.image)
                t0 = time.monotonic()
                result = await _analyze_image(image)
                processing_ms = int((time.monotonic() - t0) * 1000)
                completed = AnalysisCompletedEvent(
                    job_id=event.job_id,
                    status="completed",
                    result=result,
                )
                asyncio.create_task(_send_analytics(
                    job_id=event.job_id,
                    tags=result.tags,
                    emotion=result.emotion,
                    ocr_len=len(result.ocr_text),
                    processing_ms=processing_ms,
                ))
            except Exception as exc:  # noqa: BLE001
                job_id = "unknown"
                try:
                    payload = json.loads(message.value.decode("utf-8"))
                    job_id = str(payload.get("job_id", "unknown"))
                except Exception:  # noqa: BLE001
                    pass

                completed = AnalysisCompletedEvent(
                    job_id=job_id,
                    status="failed",
                    error=str(exc),
                )

            await _publish_completed(producer, completed_topic, completed)
    finally:
        await consumer.stop()
        await producer.stop()


@app.on_event("startup")
async def startup() -> None:
    await _init_clickhouse()
    app.state.kafka_task = asyncio.create_task(_consume_analysis_requests())


@app.on_event("shutdown")
async def shutdown() -> None:
    task = getattr(app.state, "kafka_task", None)
    if task is None:
        return

    task.cancel()
    try:
        await task
    except asyncio.CancelledError:
        pass


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

    return await _analyze_image(image)


if __name__ == "__main__":
    # Для локального запуска: python3 main.py
    import uvicorn

    port = int(os.getenv("PORT", "8000"))
    uvicorn.run(app, host="0.0.0.0", port=port)
