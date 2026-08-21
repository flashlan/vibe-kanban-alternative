FROM python:3.11-slim

WORKDIR /app

# Install CPU-only torch first (no NVIDIA/CUDA libs) to keep the image small.
RUN pip install --no-cache-dir torch --index-url https://download.pytorch.org/whl/cpu

# Install the rest (sentence-transformers reuses the already-installed torch).
COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

COPY app.py ./

# Pre-download the model at build time so first request is fast.
# all-MiniLM-L6-v2 is 384-dim → matches EMBED_DIM=384.
# Override with --build-arg ST_MODEL=... (and set EMBED_DIM to match its dim).
ARG ST_MODEL=sentence-transformers/all-MiniLM-L6-v2
ENV ST_MODEL=${ST_MODEL} \
    HF_HUB_OFFLINE=0
RUN python -c "from sentence_transformers import SentenceTransformer; SentenceTransformer('${ST_MODEL}', device='cpu')"

EXPOSE 8001

CMD ["python", "app.py"]
