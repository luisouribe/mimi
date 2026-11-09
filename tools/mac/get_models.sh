#!/bin/sh
# Downloads Mimi's three speech models into models/ and checks each file against the
# copies used on the Windows PC. About 1.4 GB in total.
# Usage, from the project folder: sh tools/mac/get_models.sh
set -eu
cd "$(dirname "$0")/../.."

PARAKEET=https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main
WHISPER=https://huggingface.co/ggerganov/whisper.cpp/resolve/main

fetch() { # url, destination, expected sha256
  if [ -f "$2" ] && [ "$(shasum -a 256 "$2" | cut -c1-64)" = "$3" ]; then
    echo "already here: $2"
    return
  fi
  mkdir -p "$(dirname "$2")"
  echo "downloading $2"
  curl -L --fail --progress-bar -o "$2.part" "$1"
  mv "$2.part" "$2"
  actual=$(shasum -a 256 "$2" | cut -c1-64)
  if [ "$actual" != "$3" ]; then
    echo "WRONG FILE: $2 is $actual, expected $3" >&2
    exit 1
  fi
}

P=models/parakeet-tdt-0.6b-v3-int8
fetch "$PARAKEET/config.json" "$P/config.json" 666903c76b9798caf2c210afd4f6cd60b08a8dbf9800ec8d7a3bc0d2148ac466
fetch "$PARAKEET/vocab.txt" "$P/vocab.txt" d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d
fetch "$PARAKEET/nemo128.onnx" "$P/nemo128.onnx" a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f
fetch "$PARAKEET/decoder_joint-model.int8.onnx" "$P/decoder_joint-model.int8.onnx" eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70
fetch "$PARAKEET/encoder-model.int8.onnx" "$P/encoder-model.int8.onnx" 6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09

fetch "$WHISPER/ggml-large-v3-turbo-q5_0.bin" models/whisper/ggml-large-v3-turbo-q5_0.bin 394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2
fetch "$WHISPER/ggml-small-q5_1.bin" models/whisper/ggml-small-q5_1.bin ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb

echo "All models are in place and match the Windows copies."
