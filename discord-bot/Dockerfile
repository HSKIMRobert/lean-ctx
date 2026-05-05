FROM python:3.13-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY bot.py spam_filter.py docs_index.py llm.py ./
COPY knowledge/ ./knowledge/

CMD ["python", "bot.py"]
