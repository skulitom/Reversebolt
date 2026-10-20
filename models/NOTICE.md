# Ocrs model attribution

The optional, unmodified text detection and text recognition weights are from
Robert Knight and the Ocrs project:

- Model card: https://huggingface.co/robertknight/ocrs
- Training project: https://github.com/robertknight/ocrs-models
- License: Creative Commons Attribution-ShareAlike 4.0 International
  (https://creativecommons.org/licenses/by-sa/4.0/)

The model card declares CC-BY-SA-4.0. The exact upstream download URLs, sizes and
SHA-256 digests are pinned in `manifest.json`. These are the ONNX files fetched
by the Ocrs 0.13.1 CLI. Reversebolt has not fine-tuned or modified them.

The model weights have their own license; Reversebolt's original source is MIT.
The Ocrs and RTen Rust libraries are separately licensed MIT OR Apache-2.0.
This attribution does not imply endorsement by their authors.

Run `scripts/get-models.ps1` from the repository to obtain the pinned files.
No models or images are downloaded/uploaded by the running desktop application.
Images are processed locally. A missing or mismatched model leaves Windows OCR
available with a visible warning. The publisher copies these notices and the
model manifest alongside the optional weights.
